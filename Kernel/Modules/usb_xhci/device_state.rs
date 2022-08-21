// 
use ::kernel::lib::mem::Box;
use crate::HostInner;
use crate::hw;
use crate::command_ring;

pub struct DeviceInfo
{
    /// Device context handle
    dc_handle: DeviceContextHandle,
    /// Number of outstanding references to this device (i.e. open endpoints)
    refcount: u8,

    // TODO: endpoint transfer rings?
    // - 16 bytes per entry, and want at least 3 entries per control transaction
    // - so, allocate 16 entries each? - OR, could just allocate a whole page per ring - sharing code with the command ring
    // NOTE: The last entry in each must be a TR_LINK back to the start
    endpoint_ring_allocs: [Option<::kernel::memory::virt::ArrayHandle<crate::hw::structs::Trb>>; 31],
    endpoint_ring_offsets: [u8; 31],    // 256*16 = 4096 (aka 1 page)
    endpoint_ring_cycles: [bool; 31],

    configuration: (u16, u16),
}

#[derive(Default)]
pub struct EnumState
{
    info: ::kernel::sync::Mutex<EnumStateDeviceInfo>,
}
#[derive(Default)]
struct EnumStateDeviceInfo
{
    route_string: u32,
    parent_info: u16,
    speed: u8,
    root_port: u8,
}
/// Device enumeration handling
impl HostInner
{
    /// Inform the driver of the location of the next device to be enumerated
    pub(crate) fn set_device_info(&self, hub_dev: Option<u8>, port: usize, speed: u8)
    {
        let (route_string, root_port, parent_info) = if let Some(v) = hub_dev
            {
                let lh = self.devices[v as usize - 1].lock();
                let dc = lh.as_ref().unwrap();
                // Get the path from the source context
                let parent_sc = self.slot_context( &dc.dc_handle );
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
                            (dc.dc_handle.slot_idx() as u16) << 8 | port as u16
                        }
                    }
                    else {
                        // USB3+
                        0
                    };
                (route_string, root_port, parent_info)
            }
            else {
                (0, port as u8 + 1, 0)
            };

        *self.enum_state.info.lock() = EnumStateDeviceInfo {
            route_string,
            parent_info,
            root_port,
            speed,
            };
    }

    /// Set a device address, and prepare the slot
    pub(crate) async fn set_address(&self, address: u8) -> Result<(),::kernel::memory::virt::MapError>
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
        let ep0_queue = {
            let mut h = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?.into_array();
            let trb_link = hw::structs::TrbLink { addr: ::kernel::memory::virt::get_phys(&h[0]), chain: false, interrupter_target: 0, ioc: false, toggle_cycle: true };
            h[TRBS_PER_PAGE as usize -1] = hw::structs::IntoTrb::into_trb(trb_link, true);
            h
            };
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
            input_context.ep0.tr_dequeue_ptr = ::kernel::memory::virt::get_phys(&ep0_queue[0]) | 1;
        }

        // TODO: Initialise the endpoints

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

        self.command_ring.lock().enqueue_command(&self.regs, command_ring::Command::SetTrDequeuePointer {
            cycle: false,
            new_dequeue_pointer: ::kernel::memory::virt::get_phys(&ep0_queue[0]),
            stream_id: 0,
            stream_context_type: 0,
            endpoint_id: 1,
            slot_idx: device_slot.slot_idx(),
        });

        // Device is now ready, with just one endpoint initialised
        // - Now need to monitor for the `set_configuration` command, and prepare the endpoints described within
        // - Monitor for:
        //   > GET_DESCRIPTOR w/ ty=2 (this is the configuration descriptor), and save the last one seen
        //   > a SET_CONFIGURATION message (not yet sent) OR just any attempt to make a new endpoing
        log_debug!("set_address({}): slot_idx={} - slot_context={:x?}", address, slot_idx, self.slot_context(&device_slot));

        // Save the endpoint handle against the device address (against the address chosen by the caller)
        let mut slot = self.devices[address as usize - 1].lock();
        assert!(slot.is_none(), "`usb_core` double-allocated a device address on this bus ({})", address);
        *slot = Some(Box::new(DeviceInfo {
            dc_handle: device_slot,
            refcount: 0,
            endpoint_ring_allocs: [
                Some(ep0_queue),
                None,None,None,None,None,
                None,None,None,None,None,
                None,None,None,None,None,
                None,None,None,None,None,
                None,None,None,None,None,
                None,None,None,None,None,
                ],
            endpoint_ring_offsets: [0; 31],
            endpoint_ring_cycles: [true; 31],
            configuration: (0,0),
            }));

        Ok( () )
    }

    pub fn set_configuration_info(&self, addr: u8, cfg: u8, endpoints_in: u16, endpoints_out: u16) {
        assert!(cfg == 0, "TODO: Handle alternative configurations");
        self.devices[addr as usize - 1].lock().as_mut().unwrap().configuration = (endpoints_in, endpoints_out);
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
    // fn release_device_context(&self, h: DeviceContextHandle) {
    //     self.memory_pools.release(h.0)
    // }
}

/// Handle to an allocated device context on a controller
// Device context is used by the controller to inform the driver of the state of the device.
pub(crate) struct DeviceContextHandle(crate::memory_pools::PoolHandle, u8);
impl DeviceContextHandle {
    pub(crate) fn pool_handle(&self) -> &crate::memory_pools::PoolHandle { &self.0 }
    pub(crate) fn slot_idx(&self) -> u8 { self.1 }
}
impl HostInner {
    /// Slot context : Used for the controller to tell the driver about device state
    fn slot_context<'a>(&'a self, handle: &'a DeviceContextHandle) -> &'a hw::structs::SlotContext {
        unsafe { &*(self.memory_pools.get(&handle.0) as *const hw::structs::SlotContext) }
    }
    // /// Endpoint context : Controls comms for the endpoint
    // fn endpoint_context<'a>(&'a self, handle: &'a DeviceContextHandle, index: u8) -> &'a hw::structs::EndpointContext {
    //     assert!(1 + index < handle.0.len() as u8);
    //     unsafe { &*(self.memory_pools.get(&handle.0).offset(1 + index as isize) as *const hw::structs::EndpointContext) }
    // }
}

impl HostInner {    
    pub(crate) fn push_ep_trbs(&self, addr: u8, index: u8) -> PushTrbState {
        let mut lh = self.devices[addr as usize - 1].lock();
        let dev = match lh.as_mut() { Some(v) => v, _ => panic!(""), };
        assert!(dev.get_endpoint(index).is_some(), "Endpoint {} of device {} not initialised", addr, index);
        PushTrbState { host: self, lh, index, count: 0 }
    }
    pub(crate) async fn wait_for_completion(&self, addr: u8, index: u8) -> (u32, u8, bool) {
        let slot_idx = {
            let mut lh = self.devices[addr as usize - 1].lock();
            let dev = match lh.as_mut() { Some(v) => v, _ => panic!(""), };
            dev.dc_handle.slot_idx()
            };
        
        let (_addr, len, cc, unk) = self.slot_events[slot_idx as usize - 1][index as usize - 1].wait().await;
        (len, cc, unk)
    }
}
pub struct PushTrbState<'a> {
    host: &'a HostInner,
    lh: ::kernel::sync::mutex::HeldMutex<'a, Option<Box<DeviceInfo>>>,
    index: u8,
    count: u8,
}
const TRBS_PER_PAGE: u8 = (0x1000/32) as u8;
impl<'a> PushTrbState<'a>
{
    /// UNSAFE: The caller must ensure that the TRB content is valid (as it might contain addresses for the hardware)
    pub(crate) unsafe fn push<T: hw::structs::IntoTrb>(&mut self, v: T) {
        self.push_inner(v.into_trb(false))
    }
    fn push_inner(&mut self, mut trb: hw::structs::Trb) {
        let (alloc, cycle, ofs) = self.lh.as_mut().unwrap().get_endpoint(self.index).unwrap();
        let (cycle, ofs) = Self::get_cycle_and_ofs(*cycle, *ofs, self.count);
        trb.set_cycle(!cycle);  // Set the cycle to the opposite of current (ensuring that this entry isn't considered... yet)
        alloc[ofs as usize] = trb;
        self.count += 1;
    }

    fn get_cycle_and_ofs(cycle: bool, ofs: u8, rel_idx: u8) -> (bool, u8) {
        if ofs + rel_idx >= TRBS_PER_PAGE-1 {
            (!cycle, ofs + rel_idx - TRBS_PER_PAGE-1)
        }
        else {
            (cycle, ofs + rel_idx)
        }
    }
}
impl<'a> ::core::ops::Drop for PushTrbState<'a> {
    fn drop(&mut self) {
        let slot_idx = self.lh.as_ref().unwrap().dc_handle.slot_idx();
        let (alloc, cycle, ofs) = self.lh.as_mut().unwrap().get_endpoint(self.index).unwrap();
        // Set the cycle bits to match the expected
        // - Do this in reverse order, so the hardware sees all new items at once
        for i in (0 .. self.count).rev() {
            let (cycle, ofs) = Self::get_cycle_and_ofs(*cycle, *ofs, i);
            // Also need to update the cycle bit on the chaining link entry
            if ofs == TRBS_PER_PAGE-2 {
                alloc[ofs as usize + 1].set_cycle(cycle);
            }
            alloc[ofs as usize].set_cycle(cycle);
        }
        let new_co = Self::get_cycle_and_ofs(*cycle, *ofs, self.count);
        log_debug!("PushTrbState::drop: {}/{} {},{} to {},{}", slot_idx, self.index, *cycle,*ofs, new_co.0, new_co.1);
        (*cycle, *ofs) = new_co;

        // Ring the doorbell
        self.host.regs.ring_doorbell(slot_idx, self.index as u32);
    }
}
impl DeviceInfo {
    fn get_endpoint(&mut self, endpoint_idx: u8) -> Option<(&mut ::kernel::memory::virt::ArrayHandle<crate::hw::structs::Trb>, &mut bool, &mut u8)> {
        assert!(endpoint_idx > 0);
        let i = endpoint_idx as usize - 1;
        Some( (
            self.endpoint_ring_allocs[i].as_mut()?,
            &mut self.endpoint_ring_cycles[i],
            &mut self.endpoint_ring_offsets[i],
        ))
    }
}