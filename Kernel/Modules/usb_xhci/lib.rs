// "Tifflin" Kernel - OHCI USB driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_xhci/lib.rs
//! eXtensible Host Controller Interface (xHCI) driver
#![no_std]
#![feature(linkage)]	// for module_define!
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

fn init()
{
	static PCI_DRIVER: pci::PciDriver = pci::PciDriver;
	::kernel::device_manager::register_driver(&PCI_DRIVER);
}

type HostRef = ArefBorrow<HostInner>;
struct HostInner {
    regs: hw::Regs,
    /// Device context pointers, only the first half is used (256 * 8 = 0x800). Second half is pool zero for the device contexts
    dcbaa: ::kernel::memory::virt::ArrayHandle<u64>,
    device_context_pool: [Option<::kernel::memory::virt::ArrayHandle<[u32; 0x20/4]>>; 4],

    command_ring: ::kernel::sync::Mutex<command_ring::CommandRing>,
    event_ring_zero: event_ring::EventRing<event_ring::Zero>,

    port_update: [::core::sync::atomic::AtomicU32; 256/32],
	port_update_waker: ::kernel::sync::Spinlock<::core::task::Waker>,

	_irq_handle: Option<::kernel::irqs::ObjectHandle>,
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
            dcbaa,
            device_context_pool: [None,None,None,None],
            command_ring: ::kernel::sync::Mutex::new(command_ring),
            event_ring_zero,
            _irq_handle: None,
            
            port_update_waker: ::kernel::sync::Spinlock::new(kernel::futures::null_waker()),
            port_update: Default::default(),
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
        log_debug!("USBSTS {:#x}", rv.regs.usbsts());
        unsafe {
            rv.regs.write_usbcmd(hw::regs::USBCMD_RS|hw::regs::USBCMD_INTE);
        }
        log_debug!("USBSTS {:#x}", rv.regs.usbsts());
        
        // TODO: Determine how to prepare the already-connected ports
        // - Should they generate an interrupt when interrupts are enabled? or just pre-scan?
        for p in 0 .. rv.regs.max_ports()
        {
            log_debug!("{:#x}", rv.regs.port(p).sc());
        }
        
        ::usb_core::register_host(Box::new(usb_host::UsbHost { host: rv.borrow() }), nports);
        
        // Test commands
        rv.command_ring.lock().enqueue_command(&rv.regs, command_ring::Command::Nop);
        rv.event_ring_zero.wait_sync();
        while let Some(_v) = rv.event_ring_zero.poll() {
        }

        Ok(rv)
    }

    fn handle_irq(&self) -> bool
    {
        let sts = self.regs.usbsts();
        if sts & (hw::regs::USBSTS_EINT|hw::regs::USBSTS_HCE|hw::regs::USBSTS_PCD) != 0 {
            log_trace!("USBSTS = {:#x}", sts);
            if sts & hw::regs::USBSTS_EINT != 0 {
                // Signal any waiting
                self.event_ring_zero.check_int(&self.regs);
            }
            true
        }
        else {
            false
        }
    }
}

/// Device enumeration handling
impl HostInner {
    fn set_address(&self, addr: u8) {
        todo!("set_address")
    }
}

/// Device contexts
impl HostInner {
    fn alloc_device_context(&self, n_endpoints: usize) -> DeviceContextHandle {
        // Find an empty slot (bitmap? or just inspecting read-only fields from the context)
        todo!("alloc_device_context");
    }
    fn release_device_context(&self, h: DeviceContextHandle) {
        todo!("release_device_context");
    }
}
/// Handle to an allocated device context on a controller
// Device context is used by the controller to inform the driver of the state of the device.
struct DeviceContextHandle {
    /// Pool index
    pool: u8,
    /// Index into the pool (one page has 128 possible 32-byte slots)
    index: u8,
    /// 
    n_endpoints: u8,
}
impl HostInner {
    fn get_device_context(&self, handle: &DeviceContextHandle) -> *const [u32; 0x20 / 4] {
        &self.device_context_pool[handle.pool as usize].as_ref().unwrap()[handle.index as usize * 8]
    }
    /// Slot context : Used for the controller to tell the driver about device state
    fn slot_context<'a>(&'a self, handle: &'a DeviceContextHandle) -> &'a hw::structs::SlotContext {
        unsafe { &*(self.get_device_context(handle) as *const hw::structs::SlotContext) }
    }

    /// Endpoint context : Controls comms for the endpoint
    fn endpoint_context<'a>(&'a self, handle: &'a DeviceContextHandle, index: u8) -> &'a hw::structs::EndpointContext {
        assert!(index < 1 + handle.n_endpoints * 2);
        unsafe { &*(self.get_device_context(handle).offset(index as isize) as *const hw::structs::EndpointContext) }
    }
}
