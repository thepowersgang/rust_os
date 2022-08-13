// "Tifflin" Kernel - OHCI USB driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_xhci/lib.rs
//! eXtensible Host Controller Interface (xHCI) driver
#![no_std]
#![feature(linkage)]	// for module_define!
#![feature(ptr_metadata)]	// for hacks
use kernel::prelude::*;
use kernel::lib::mem::aref::{Aref,ArefBorrow};

#[macro_use]
extern crate kernel;
extern crate usb_core;

::kernel::module_define!(usb_xhci, [usb_core], init);

mod pci;
mod hw;
mod usb_host;
mod command_ring;
mod event_ring;
mod memory_pools;

fn init()
{
	static PCI_DRIVER: pci::PciDriver = pci::PciDriver;
	::kernel::device_manager::register_driver(&PCI_DRIVER);
}

#[derive(Default)]
struct AtomicBitset256([::core::sync::atomic::AtomicU32; 256/32]);
impl AtomicBitset256 {
    pub fn set(&self, idx: u8) {
        use ::core::sync::atomic::Ordering;
        self.0[idx as usize / 32].fetch_or(1 << idx%32, Ordering::SeqCst);
    }
    pub fn get_first_set_and_clear(&self) -> Option<usize> {
        use ::core::sync::atomic::Ordering;
        for (i,s) in self.0.iter().enumerate() {
            let v = s.load(Ordering::SeqCst);
            if v != 0 {
                let j = v.trailing_zeros() as usize;
                let bit = 1 << j;
                if s.fetch_and(!bit, Ordering::SeqCst) & bit != 0 {
                    return Some(i*32 + j);
                }
            }
        }
        None
    }
    #[cfg(false_)]
    pub fn iter_set_and_clear(&self) -> impl Iterator<Item=usize> + '_ {
        use ::core::sync::atomic::Ordering;
        self.0.iter().enumerate()
            // Load/clear each word
            .map(|(i,v)| (i,v.swap(0, Ordering::SeqCst)))
            // Convert into (Index,IsSet)
            .flat_map(|(j,v)| (0..32).map(move |i| (j*32+i, (v >> i) & 1 != 0) ))
            // Yield only the indexes of set bits
            .filter_map(|(idx,isset)| if isset { Some(idx) } else { None })
    }
}

type HostRef = ArefBorrow<HostInner>;
struct HostInner {
    regs: hw::Regs,

    memory_pools: memory_pools::MemoryPools,

    command_ring: ::kernel::sync::Mutex<command_ring::CommandRing>,
    event_ring_zero: event_ring::EventRing<event_ring::Zero>,

    port_update: AtomicBitset256,
	port_update_waker: ::kernel::sync::Spinlock<::core::task::Waker>,

    slot_enable_ready: ::kernel::futures::flag::SingleFlag,
    slot_enable_idx: ::core::sync::atomic::AtomicU8,

	_irq_handle: Option<::kernel::irqs::ObjectHandle>,

    enum_state: EnumState,
}
impl HostInner
{
    /// Construct a new instance
    fn new_aref(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Aref<Self>, ::kernel::device_manager::DriverBindError>
    {
        log_debug!("new_boxed(irq={irq}, io={io:?}");
        // SAFE: This function is only called with a valid register binding
        let regs = unsafe { hw::Regs::new(io) };

        // Controller init
        // - Trigger a reset (via PCI?) and wait for USBSTS.NCR to become zero
        unsafe {
            regs.write_usbcmd(hw::regs::USBCMD_HCRST);
        }
        while regs.usbsts() & hw::regs::USBSTS_CNR != 0 {
            //::kernel::time::
            // TODO: Sleep with timeout
        }
        // - Device init, any order:
        //   - Configure CONFIG.MaxSlotsEn to the number of device slots desired
        let n_device_slots: u8 = 128;
        //   - Set DCBAAP to the device context array
        let mut dcbaa = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?.into_array();
        //     > Entry zero points to an array of scratchpad pages, see the `Max Scratchpad Buffers Hi/Lo` fields in HCSPARAMS2 TODO check s4.20 of the spec
        if regs.max_scratchpad_buffers() > 0
        {
            let array = &mut dcbaa[256..];
            let mut scratchpad_entries = Vec::with_capacity(regs.max_scratchpad_buffers() as usize);
            for i in 0 .. regs.max_scratchpad_buffers() as usize
            {
                let e = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?;
                array[i] = ::kernel::memory::virt::get_phys( e.as_ref::<()>(0) ) as u64;
                scratchpad_entries.push(e);
            }
            dcbaa[0] = ::kernel::memory::virt::get_phys(array) as u64;
        }
        unsafe {
            regs.set_dcbaap(::kernel::memory::virt::get_phys(dcbaa.as_ptr()) as u64);
            regs.write_config(n_device_slots as u32);
        }
        //   - Set the command ring (Set the initial dequeue pointer?)
        let command_ring = command_ring::CommandRing::new(&regs)?;
        //   - Set up MSI-X (aka the event ring)
        let event_ring_zero = event_ring::EventRing::new_zero(&regs)?;

        let nports = regs.max_ports();
        let mut rv = Aref::new(HostInner {
            regs,
            memory_pools: memory_pools::MemoryPools::new(dcbaa),

            command_ring: ::kernel::sync::Mutex::new(command_ring),
            event_ring_zero,
            _irq_handle: None,
            
            port_update_waker: ::kernel::sync::Spinlock::new(kernel::futures::null_waker()),
            port_update: Default::default(),

            slot_enable_ready: Default::default(),
            slot_enable_idx: Default::default(),
            enum_state: Default::default(),
            });

        // Bind interrupt
        {
            struct RawSend<T: Send>(*const T);
            unsafe impl<T: Send> Send for RawSend<T> {}
            let ret_raw = RawSend(&*rv);
            // SAFE: Pointer _should_ be valid as long as this IRQ binding exists
            let binding = ::kernel::irqs::bind_object(irq, Box::new(move || unsafe { (*ret_raw.0).handle_irq() } ));
            Aref::get_mut(&mut rv).unwrap()._irq_handle = Some(binding);
        }
            
        // - Set USBCMD.RUN = 1
        log_debug!("pre-start: USBSTS {:#x}", rv.regs.usbsts());
        unsafe {
            rv.regs.write_usbcmd(hw::regs::USBCMD_RS|hw::regs::USBCMD_INTE);
        }
        log_debug!("Post-run: USBSTS {:#x}", rv.regs.usbsts());
        
        ::usb_core::register_host(Box::new(usb_host::UsbHost { host: rv.borrow() }), nports);

        // Test commands
        if false {
            log_debug!("--- TESTING COMMAND ---");
            rv.command_ring.lock().enqueue_command(&rv.regs, command_ring::Command::Nop);
            rv.command_ring.lock().enqueue_command(&rv.regs, command_ring::Command::Nop);
            rv.command_ring.lock().enqueue_command(&rv.regs, command_ring::Command::Nop);
            ::kernel::threads::yield_time();
            log_debug!("-/- TESTING COMMAND -/-");
        }
        
        // TODO: Determine how to prepare the already-connected ports
        // - Should they generate an interrupt when interrupts are enabled? or just pre-scan?
        for p in 0 .. rv.regs.max_ports()
        {
            log_debug!("Port Status #{}: {:#x}", 1+p, rv.regs.port(p).sc());
            if rv.regs.port(p).sc() & hw::regs::PORTSC_CCS != 0 {
                rv.port_update.set(p);
            }
        }
        rv.port_update_waker.lock().wake_by_ref();

        Ok(rv)
    }

    fn handle_irq(&self) -> bool
    {
        let sts = self.regs.usbsts();
        log_trace!("USBSTS = {:#x}", sts);
        if sts & (hw::regs::USBSTS_EINT|hw::regs::USBSTS_HCE|hw::regs::USBSTS_PCD) != 0 {
            let mut h = 0;
            if sts & hw::regs::USBSTS_EINT != 0 {
                h |= hw::regs::USBSTS_EINT;
                // Signal any waiting
                self.event_ring_zero.check_int(&self.regs);
                while let Some(ev) = self.event_ring_zero.poll(&self.regs) {
                    use event_ring::Event;
                    match ev
                    {
                    Event::PortStatusChange { port_id, completion_code: _ } => {
                        // Port IDs are indexed from 1
                        self.port_update.set(port_id - 1);
                        self.port_update_waker.lock().wake_by_ref();
                        },
                    Event::CommandCompletion { trb_pointer, completion_code, param, slot_id, vf_id } => {
                        let ty = self.command_ring.lock().get_command_type(trb_pointer);
                        if completion_code == 1 {
                            log_trace!("CommandCompletion {:#x} {:?}: SUCCESS", trb_pointer, ty);
                            match ty
                            {
                            Some(hw::structs::TrbType::NoOpCommand) => {
                                },
                            Some(hw::structs::TrbType::EnableSlotCommand) => {
                                self.slot_enable_idx.store(slot_id, ::core::sync::atomic::Ordering::SeqCst);
                                self.slot_enable_ready.trigger();
                                },
                            Some(hw::structs::TrbType::AddressDeviceCommand) => {
                                self.slot_enable_ready.trigger();
                                }
                            _ => {},
                            }
                        }
                        else {
                            log_error!("CommandCompletion {:#x} {:?}: Not success, {}", trb_pointer, ty, completion_code);
                        }
                        },
                    _ => {},
                    }
                }
            }
            if h != sts {
                todo!("Unhandled interrupt bit");
            }
            self.regs.write_usbsts(h);
            true
        }
        else {
            false
        }
    }
}


#[derive(Default)]
struct EnumState
{
    info: ::kernel::sync::Mutex<EnumStateDeviceInfo>,
}
#[derive(Default)]
struct EnumStateDeviceInfo {
    route_string: u32,
    parent_info: u16,
    speed: u8,
    root_port: u8,
}
/// Device enumeration handling
impl HostInner
{
    fn set_device_info(&self, hub_dev: Option<&DeviceContextHandle>, port: usize, speed: u8)
    {
        let (route_string, root_port, parent_info) = if let Some(v) = hub_dev
            {
                // Get the path from the source context
                let parent_sc = self.slot_context(v);
                let route_string = {
                    let parent_rs = parent_sc.word0 & 0xF_FFFF;
                    let shift = 32 - parent_rs.leading_zeros();
                    parent_rs | (port << shift) as u32
                    };
                let root_port = (parent_sc.word1 >> 16) as u8;
                // If the parent is USB3 and this is earlier, then set to the parent's slot ID and this port
                let parent_info = if speed <= 3 {
                        if (parent_sc.word0 >> 20) & 0xF > 3  {
                            (parent_sc.word2 & 0xFFFF) as u16
                        }
                        else {
                            (v.slot_idx() as u16) << 8 | port as u16
                        }
                    }
                    else {
                        // USB3+
                        0
                    };
                (route_string, root_port, parent_info)
            }
            else {
                (0, port as u8, 0)
            };

        *self.enum_state.info.lock() = EnumStateDeviceInfo {
            route_string,
            parent_info,
            root_port,
            speed,
            };
    }
    async fn set_address(&self, address: u8)
    {
        // Request the hardware allocate a slot
        self.slot_enable_ready.reset();
        self.command_ring.lock().enqueue_command(&self.regs, command_ring::Command::EnableSlot);
        // Wait for the newly allocated to slot to be availble.
        self.slot_enable_ready.wait().await;
        // Get the assigned slot index (won't be clobbered, as this runs with `usb_core`'s dev0 lock)
        let slot_idx = self.slot_enable_idx.load(::core::sync::atomic::Ordering::SeqCst);

        #[repr(C)]
        struct AddrInputContext {
            ctrl: hw::structs::InputControlContext,
            slot: hw::structs::SlotContext,
            ep0: hw::structs::EndpointContext,
        }
        // Prepare the device slot
        let device_slot = self.alloc_device_context(slot_idx, 1);
        // Create an input context (6.2.2.1)
        let input_context_h = self.memory_pools.alloc(2 + 1).expect("Out of space for input context");
        {
            let input_context: &mut AddrInputContext = unsafe { &mut *(self.memory_pools.get(&input_context_h) as *const _ as *mut _) };
            let info = self.enum_state.info.lock();
            input_context.ctrl = hw::structs::InputControlContext::zeroed();
            input_context.ctrl.add_context_flags = 0b11;
            input_context.slot = hw::structs::SlotContext::new([
                0   | info.route_string  // Valid route string
                    | (info.speed as u32) << 20 // Speed of the device
                    | (1 << 27) // "Context Entries" = 1 (i.e. just EP0)
                    ,
                0   | (info.root_port as u32) << 16 // Root hub port number
                    ,
                0   | (info.parent_info as u32) << 0    // Slot ID and port number of the parent (USB2/3) hub
                    ,
                0,
                ]);
            input_context.ep0 = hw::structs::EndpointContext::zeroed();
        }

        // Issue the `AddressDevice` command
        self.memory_pools.set_dcba(&device_slot);
        self.command_ring.lock().enqueue_command(&self.regs, command_ring::Command::AddressDevice {
            slot_idx,
            block_set_address: false,
            input_context_pointer: self.memory_pools.get_phys(&input_context_h),
            });
        // Wait for a "Command Complete" event
        self.slot_enable_ready.wait().await;
        self.memory_pools.release(input_context_h);
        todo!("set_address({}): slot_idx={} - slot_context={:?}", address, slot_idx, self.slot_context(&device_slot));
    }
}

/// Device contexts
impl HostInner {
    fn alloc_device_context(&self, slot_index: u8, n_endpoints: usize) -> DeviceContextHandle {
        use core::convert::TryInto;
        // Find an empty slot (bitmap? or just inspecting read-only fields from the context)
        let n_blocks = (1 + n_endpoints).try_into().expect("Too many blocks");
        DeviceContextHandle( self.memory_pools.alloc(n_blocks).expect("Out of space for a device context"), slot_index )
    }
    fn release_device_context(&self, h: DeviceContextHandle) {
        self.memory_pools.release(h.0)
    }
}
/// Handle to an allocated device context on a controller
// Device context is used by the controller to inform the driver of the state of the device.
struct DeviceContextHandle(memory_pools::PoolHandle, u8);
impl DeviceContextHandle {
    fn slot_idx(&self) -> u8 { self.1 }
}
impl HostInner {
    /// Slot context : Used for the controller to tell the driver about device state
    fn slot_context<'a>(&'a self, handle: &'a DeviceContextHandle) -> &'a hw::structs::SlotContext {
        unsafe { &*(self.memory_pools.get(&handle.0) as *const hw::structs::SlotContext) }
    }
    /// Endpoint context : Controls comms for the endpoint
    fn endpoint_context<'a>(&'a self, handle: &'a DeviceContextHandle, index: u8) -> &'a hw::structs::EndpointContext {
        assert!(1 + index < handle.0.len() as u8);
        unsafe { &*(self.memory_pools.get(&handle.0).offset(1 + index as isize) as *const hw::structs::EndpointContext) }
    }
}
