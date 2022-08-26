use ::usb_core::host;
use crate::hw::structs as hw_structs;

type Error = ::kernel::memory::virt::MapError;

pub struct Control
{
    host: crate::HostRef,
    pub(super) addr: u8,
    endpoint: u8,
    // Needs the handle to the device
    // And the endpoint number
    // TODO: Does this need the rings (or should they be controlled by the host)
}
pub struct Endpoint0
{
    inner: Control,
}

impl Control
{
    pub(crate) fn new(host: crate::HostRef, addr: u8, endpoint: u8, max_packet_size: usize) -> Result<Self,Error> {
        if endpoint == 0 {
            host.claim_endpoint(addr, 1, hw_structs::EndpointType::Control, max_packet_size)?;
        }
        else {
            host.claim_endpoint(addr, endpoint * 2 + 0, hw_structs::EndpointType::Control, max_packet_size)?;
            host.claim_endpoint(addr, endpoint * 2 + 1, hw_structs::EndpointType::Control,max_packet_size)?;
        }
        Ok(Control { host, addr, endpoint })
    }
}
impl ::core::ops::Drop for Control {
    fn drop(&mut self) {
        if self.endpoint == 0 {
            self.host.release_endpoint(self.addr, 1);
        }
        else {
            self.host.release_endpoint(self.addr, self.endpoint * 2 + 0);
            self.host.release_endpoint(self.addr, self.endpoint * 2 + 1);
        }
    }
}

impl Endpoint0
{
    pub(crate) fn new(host: crate::HostRef, addr: u8, max_packet_size: usize) -> Result<Self,Error> {
        Ok(Endpoint0 { inner: Control::new(host, addr, 0, max_packet_size)? })
    }
}

fn parse_setup(setup_data: &[u8], transfer_type: hw_structs::TrbControlSetupTransferType) -> hw_structs::TrbControlSetup {
    use core::convert::TryInto;
    hw_structs::TrbControlSetup {
        bm_request_type: setup_data[0],
        b_request: setup_data[1],
        w_value: u16::from_le_bytes( setup_data[2..][..2].try_into().unwrap() ),
        w_index: u16::from_le_bytes( setup_data[4..][..2].try_into().unwrap() ),
        w_length: u16::from_le_bytes( setup_data[6..][..2].try_into().unwrap() ),
        interupter_target: 0,
        trb_transfer_length: 8,
        transfer_type,
        ioc: false,
        idt: true, // "..the Parameter component of this TRB contains Setup Data"
    }
}

fn get_data(direction_in: bool, d: hw_structs::TrbNormalData, len: u32, is_last: bool) -> hw_structs::TrbControlData {
    if let hw_structs::TrbNormalData::InlineData(_) = d {
        assert!(len <= 8);
    }
    hw_structs::TrbControlData {
        data: d,
        trb_transfer_length: len,
        chain_bit: !is_last,
        direction_in,
        evaluate_next_trb: !is_last,
        interrupt_on_short_packet: false,
        ioc: false,
        no_snoop: false,
        td_size: 1, // TODO
        interrupter_target: 0,
        }
}

impl host::ControlEndpoint for Control {
    fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_trace!("out_only({:?}, {:?})", ::kernel::logging::HexDump(setup_data), ::kernel::logging::HexDump(out_data));
        let index = if self.endpoint == 0 { 1 } else { self.endpoint * 2 + 0 };
        // Create TRBs for the data (Setup, data, status)
        // Add it to the TRB for this endpoint
        {
            let mut state = self.host.push_ep_trbs(self.addr, index);
            // SAFE: No data attached
            unsafe {
                state.push(parse_setup(setup_data, crate::hw::structs::TrbControlSetupTransferType::Out));
            }
            if out_data.len() > 0
            {
                if let Some(d) = hw_structs::TrbNormalData::make_inline(out_data) {
                    // SAFE: No memory accesses
                    unsafe {
                        state.push(get_data(false, d, out_data.len() as u32, true));
                    }
                }
                else {
                    for (paddr, len, is_last) in super::iter_contigious_phys(out_data) {
                        // SAFE: Trusting ourselves to wait until the hardware is done
                        unsafe {
                            state.push(get_data(false, hw_structs::TrbNormalData::Pointer(paddr), len as u32, is_last));
                        }
                    }
                }
            }
            // SAFE: No data attached
            unsafe {
                state.push(hw_structs::TrbControlStatus { direction_in: true, ioc: true, evaluate_next_trb: false, interrupter_target: 0 });
            }
        }

        let len = out_data.len();
        let f = self.host.wait_for_completion(self.addr, index);
        super::make_asyncwaitio(async move {
            let (unused_len, completion_code) = f.await;
            log_trace!("out_only complete: {} bytes, completion_code={}", len, completion_code);
            len - unused_len as usize
        })
    }
    fn in_only<'a>(&'a self, setup_data: &'a [u8], in_data: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_debug!("in_only({:?}, {})", ::kernel::logging::HexDump(setup_data), in_data.len());
        let index = self.endpoint * 2 + 1;
        {
            let mut state = self.host.push_ep_trbs(self.addr, index);
            // SAFE: No data attached
            unsafe {
                state.push(parse_setup(setup_data, crate::hw::structs::TrbControlSetupTransferType::In));
            }
            for (paddr, len, is_last) in super::iter_contigious_phys(in_data) {
                // SAFE: Trusting ourselves to wait until the hardware is done
                unsafe {
                    state.push(get_data(true, hw_structs::TrbNormalData::Pointer(paddr), len as u32, is_last));
                }
            }
            // SAFE: No data attached
            unsafe {
                state.push(hw_structs::TrbControlStatus { direction_in: false, ioc: true, evaluate_next_trb: false, interrupter_target: 0 });
            }
        }

        let len = in_data.len();
        let f = self.host.wait_for_completion(self.addr, index);
        super::make_asyncwaitio(async move {
            let (unused_len, completion_code) = f.await;
            log_trace!("in_only complete: {} bytes, completion_code={}", len, completion_code);
            len - unused_len as usize
        })
    }
}

impl host::ControlEndpoint for Endpoint0 {
    fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        // Monitor for:
        // - SET_CONFIGURATION request (Request type 0, request number 9)
        if setup_data.len() >= 4 && &setup_data[..2] == &[0x00, 9] {
            todo!("Handle SET_CONFIGURATION")
            // This needs special handling, I think? (TODO)
            // - At least need to tell the host of the new endpoint count
        }
        self.inner.out_only(setup_data, out_data)
    }
    fn in_only<'a>(&'a self, setup_data: &'a [u8], in_data: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        // Monitor for:
        // - GET_DESCRIPTOR
        if setup_data.len() >= 8 && &setup_data[..2] == &[0x80, 6] {
            if setup_data[3] == 2 /* Descriptor_Configuration */ {
                return super::make_asyncwaitio(async move {
                    // - Send the message, but intercept the reply
                    let len = self.inner.in_only(setup_data, in_data).await;
                    let data = &in_data[..len];
                
                    //assert!(data[0] >= );  // Length
                    assert!(data[1] == 2);  // Descriptor type: Configuraton
                    let total_length = u16::from_le_bytes(::core::convert::TryFrom::try_from(&data[2..4]).unwrap());
                    if len >= total_length as usize
                    {
                        // - Decode the configuration and count the number of endpoints
                        log_trace!("Endpoint0::in_only: Descriptor_Configuration = {:?}", ::kernel::logging::HexDump(data));
                        let desc_index = setup_data[2];
                        let num_interface = data[4];
                        let mut it = ::usb_core::hw_decls::IterDescriptors(data);
                        let mut n_endpoints = 0;
                        let mut max_endpoint = 0;
                        let mut endpoints_i = 0u16;
                        let mut endpoints_o = 0u16;
                        while let Some(desc) = it.next() {
                            use ::usb_core::hw_decls::DescriptorAny;
                            if let Ok(DescriptorAny::Endpoint(ep_desc)) = DescriptorAny::from_bytes(desc) {
                                log_debug!("ep_desc.address = {:#x}", ep_desc.address);
                                let ep_num = ep_desc.address & 0xF;
                                let ep_dir_in = ep_desc.address & 0x80 != 0;
                                //let ep_type = (ep_desc.attributes & 0x3) >> 0;

                                if ep_dir_in {
                                    endpoints_i |= 1 << ep_num;
                                }
                                else {
                                    endpoints_o |= 1 << ep_num;
                                }
                                max_endpoint = ep_num * 2 + ep_dir_in as u8;
                                n_endpoints += 1;
                            }
                        }
                        // - Save endpoint count against the configuration index
                        log_debug!("Endpoint0::in_only: Configuration {} has {} interfaces w/ {} endpoints (max {})", desc_index, num_interface, n_endpoints, max_endpoint);
                        self.inner.host.set_configuration_info(self.inner.addr, desc_index, endpoints_i, endpoints_o);
                    }
                    len
                });
            }
        }
        self.inner.in_only(setup_data, in_data)
    }
}