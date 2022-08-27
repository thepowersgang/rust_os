// 
use ::kernel::lib::mem::Box;
use crate::HostInner;
use crate::hw;


pub struct DeviceInfo
{
    /// Backing memory for the device context block
    /// 
    /// - 1024 bytes reserved for the DCB (32*32)
    /// - 1056 bytes reserved for input context (32*32+32)
    /// - 2016 bytes for misc... probably endpoint0's ring?
    device_context_page: DeviceContextPage,
    
    slot_idx: u8,
    /// Bitmask of claimed/active endpoints
    ref_flags: u32,
    
    /// Is the device configured (i.e. does it have a full-sized context block)
    is_configured: bool,

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
                let parent_sc: &hw::structs::SlotContext  = dc.device_context_page.slot_context();
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
                            (dc.slot_idx as u16) << 8 | port as u16
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
        self.command_ring.lock().enqueue_command(&self.regs, hw::commands::EnableSlot);
        // Wait for the newly allocated to slot to be availble.
        self.slot_enable_ready.wait().await;
        // Get the assigned slot index (won't be clobbered, as this runs with `usb_core`'s dev0 lock)
        let slot_idx = self.slot_enable_idx.load(::core::sync::atomic::Ordering::SeqCst);

        let ep0_queue = Self::alloc_ep_queue()?;

        // Prepare the device slot
        let mut device_context_page = dcp::DeviceContextPage::new()?;
        // Create an input context (6.2.2.1)
        {
            let input_context = device_context_page.input_context_mut();
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
            input_context.eps[0] = hw::structs::EndpointContext::zeroed();
            input_context.eps[0].tr_dequeue_ptr = ::kernel::memory::virt::get_phys(&ep0_queue[0]) | 1;
            input_context.eps[0].set_word1(hw::structs::EndpointType::Control, 0);  // TODO: What's the default MPS?
        }

        // SAFE: Pointer is valid, `device_context_page` is kept as long as this struct exists
        unsafe {
            self.command_ring.lock().set_dcba(slot_idx, device_context_page.slot_context_phys());
        }

        // Issue the `AddressDevice` command
        // SAFE: The address is kept valid
        self.command_ring.lock().enqueue_command(&self.regs, unsafe { hw::commands::AddressDevice::new(slot_idx, device_context_page.input_context_phys(), false) });
        // Wait for a "Command Complete" event
        self.slot_enable_ready.wait().await;

        // Device is now ready, with just one endpoint initialised
        // - Now need to monitor for the `set_configuration` command, and prepare the endpoints described within
        // - Monitor for:
        //   > GET_DESCRIPTOR w/ ty=2 (this is the configuration descriptor), and save the last one seen
        //   > a SET_CONFIGURATION message (not yet sent) OR just any attempt to make a new endpoing
        log_debug!("set_address({}): slot_idx={} - slot_context={:x?}", address, slot_idx, device_context_page.slot_context());

        // Save the endpoint handle against the device address (against the address chosen by the caller)
        let mut slot = self.devices[address as usize - 1].lock();
        assert!(slot.is_none(), "`usb_core` double-allocated a device address on this bus ({})", address);
        *slot = Some(Box::new(DeviceInfo {
            device_context_page,
            slot_idx,
            ref_flags: 0,
            is_configured: false,
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

    /// Allocate an endpoint queue
    fn alloc_ep_queue() -> Result< ::kernel::memory::virt::ArrayHandle<crate::hw::structs::Trb>, ::kernel::memory::virt::MapError> {
        let mut h = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?.into_array();
        let trb_link = hw::structs::TrbLink { addr: ::kernel::memory::virt::get_phys(&h[0]), chain: false, interrupter_target: 0, ioc: false, toggle_cycle: true };
        h[TRBS_PER_PAGE as usize -1] = hw::structs::IntoTrb::into_trb(trb_link, true);
        Ok(h)
    }

    /// Claim (allocate) an endpoint
    pub fn claim_endpoint(&self, addr: u8, endpoint_id: u8, endpoint_type: hw::structs::EndpointType, max_packet_size: usize) -> Result<(),::kernel::memory::virt::MapError>
    {
        let mut lh = self.devices[addr as usize - 1].lock();
        let dev = lh.as_deref_mut().expect("claim_endpoint on bad address");
        assert!(endpoint_id < 32);
        assert!(dev.ref_flags & 1 << endpoint_id == 0, "Claiming claimed endpoint");
        dev.ref_flags |= 1 << endpoint_id;
        if endpoint_id == 1 {
            // Does this need to update MPS?
            return Ok(());
        }
        // If not yet configured, then ensure that there's sufficient space allocated
        if !dev.is_configured {
            let max_ep_in = 16 - dev.configuration.0.leading_zeros() as u8;
            let max_ep_out = 16 - dev.configuration.1.leading_zeros() as u8;
            let _num_ep_slots = u8::max( max_ep_in*2 + 1, max_ep_out*2 );
            // Re-address the device, thus updating the DCBA?
            dev.is_configured = true;
        }

        let ep_queue = Self::alloc_ep_queue()?;
        {
            let input_context = dev.device_context_page.input_context_mut();
            input_context.ctrl = hw::structs::InputControlContext::zeroed();
            input_context.ctrl.add_context_flags = (1 << endpoint_id) | 1;
            input_context.eps[endpoint_id as usize - 1].set_word1(endpoint_type, max_packet_size as u16);
            input_context.eps[endpoint_id as usize - 1].tr_dequeue_ptr = ::kernel::memory::virt::get_phys(&ep_queue[0]) | 1;
        }
        dev.endpoint_ring_allocs[endpoint_id as usize - 1] = Some(ep_queue);

        // SAFE: Pointer is kept valid and unchanging until the hardware is done with it (when the `.sleep()` below returns)
        self.command_ring.lock().enqueue_command(&self.regs, unsafe { hw::commands::ConfigureEndpoint::new_configure(dev.slot_idx, dev.device_context_page.input_context_phys()) });
        self.slot_events[dev.slot_idx as usize - 1].configure.sleep();

        Ok(())
    }

    /// Release/deallocate an endpoint
    pub fn release_endpoint(&self, addr: u8, endpoint_id: u8)
    {
        let mut dev = self.devices[addr as usize - 1].lock();
        let dev = dev.as_deref_mut().expect("release_endpoint on bad address");
        assert!(endpoint_id < 32);
        assert!(dev.ref_flags & 1 << endpoint_id != 0, "Releasing unclaimed endpoint");
        dev.ref_flags &= !(1 << endpoint_id);
        todo!("release_endpoint({}, {})", addr, endpoint_id);
    }

}

impl HostInner {    
    /// Start pushing TRBs to an endpoint
    pub(crate) fn push_ep_trbs(&self, addr: u8, index: u8) -> PushTrbState {
        let mut lh = self.devices[addr as usize - 1].lock();
        let dev = match lh.as_mut() { Some(v) => v, _ => panic!(""), };
        assert!(dev.get_endpoint(index).is_some(), "Endpoint {} of device {} not initialised", addr, index);
        //assert!(!self.slot_events[dev.slot_idx as usize - 1].endpoints[index as usize - 1].is_ready());
        // TODO: Get the current read position of the ring and ensure that it's not full
        PushTrbState { host: self, lh, index, count: 0 }
    }
    /// Wait until completion is raised on the endpoint
    pub(crate) async fn wait_for_completion(&self, addr: u8, index: u8) -> (u32, u8) {
        let slot_idx = {
            let mut lh = self.devices[addr as usize - 1].lock();
            let dev = match lh.as_mut() { Some(v) => v, _ => panic!(""), };
            dev.slot_idx
            };
        
        let (_addr, len, cc) = self.slot_events[slot_idx as usize - 1].endpoints[index as usize - 1].wait().await;
        (len, cc)
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
    pub(crate) unsafe fn push<T: hw::structs::TransferTrb>(&mut self, v: T) {
        self.push_inner(v.into_trb(false))
    }
    fn push_inner(&mut self, mut trb: hw::structs::Trb) {
        let (alloc, cycle, ofs) = self.lh.as_mut().unwrap().get_endpoint(self.index).unwrap();
        let (cycle, ofs) = Self::get_cycle_and_ofs(*cycle, *ofs, self.count);
        trb.set_cycle(!cycle);  // Set the cycle to the opposite of current (ensuring that this entry isn't considered... yet)
        alloc[ofs as usize] = trb;
        log_debug!("{:#x} = {:?}", ::kernel::memory::virt::get_phys(&alloc[ofs as usize]), trb);
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
        let slot_idx = self.lh.as_ref().unwrap().slot_idx;
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

use dcp::DeviceContextPage;
mod dcp {
    use crate::hw;

    pub(super) struct DeviceContextPage(::kernel::memory::virt::AllocHandle);
    impl DeviceContextPage {    
        pub(super) fn new() -> Result<Self,::kernel::memory::virt::MapError> {
            Ok( DeviceContextPage(::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?) )
        }

        pub fn slot_context(&self) -> &hw::structs::SlotContext {
            self.0.as_ref(0)
        }
        pub fn slot_context_phys(&self) -> u64 {
            ::kernel::memory::virt::get_phys(self.slot_context())
        }

        pub fn input_context(&self) -> &hw::structs::AddrInputContext {
            self.0.as_ref(1024)
        }
        pub fn input_context_mut(&mut self) -> &mut hw::structs::AddrInputContext {
            self.0.as_mut(1024)
        }
        pub fn input_context_phys(&self) -> u64 {
            ::kernel::memory::virt::get_phys(self.input_context())
        }
    }
}