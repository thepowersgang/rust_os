use ::usb_core::host;
use crate::hw::structs as hw_structs;

type Error = ::kernel::memory::virt::MapError;

pub struct BulkIn
{
    host: crate::HostRef,
    pub(super) addr: u8,
    index: u8,
}
pub struct BulkOut
{
    host: crate::HostRef,
    pub(super) addr: u8,
    index: u8,
}

impl BulkIn {
    pub(crate) fn new(host: crate::HostRef, addr: u8, endpoint: u8, max_packet_size: usize) -> Result<Self,Error> {
        let index = endpoint * 2 + 1;
        host.claim_endpoint(addr, index, hw_structs::EndpointType::BulkIn, max_packet_size)?;
        Ok(Self { host, addr, index })
    }
}
impl ::core::ops::Drop for BulkIn {
    fn drop(&mut self) {
        self.host.release_endpoint(self.addr, self.index);
    }
}
impl BulkOut {
    pub(crate) fn new(host: crate::HostRef, addr: u8, endpoint: u8, max_packet_size: usize) -> Result<Self,Error> {
        let index = endpoint * 2 + 0;
        host.claim_endpoint(addr, index, hw_structs::EndpointType::BulkOut, max_packet_size)?;
        Ok(Self { host, addr, index })
    }
}
impl ::core::ops::Drop for BulkOut {
    fn drop(&mut self) {
        self.host.release_endpoint(self.addr, self.index);
    }
}

/// Construct a semi-sensible "normal trb" for this data
fn get_data(_direction_in: bool, d: hw_structs::TrbNormalData, len: u32, is_last: bool) -> hw_structs::TrbNormal {
    if let hw_structs::TrbNormalData::InlineData(_) = d {
        assert!(len <= 8);
    }
    hw_structs::TrbNormal {
        data: d,
        transfer_length: len,
        chain_bit: !is_last,
        evaluate_next_trb: !is_last,
        interrupt_on_short_packet: false,
        ioc: is_last,
        no_snoop: false,
        td_size: 1, // TODO
        interrupter_target: 0,
        block_event_interrupt: false,
        }
}


impl host::BulkEndpointIn for BulkIn {
    fn recv<'a>(&'a self, buffer: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_debug!("recv({}:{} {})", self.addr, self.index, buffer.len());
        {
            let mut state = self.host.push_ep_trbs(self.addr, self.index);
            
            for (paddr, len, is_last) in super::iter_contigious_phys(buffer) {
                // SAFE: Trusting ourselves to wait until the hardware is done
                unsafe {
                    state.push(get_data(true, hw_structs::TrbNormalData::Pointer(paddr), len as u32, is_last));
                }
            }
        }

        let len = buffer.len();
        let f = self.host.wait_for_completion(self.addr, self.index);
        super::make_asyncwaitio(async move {
            let (unused_len, completion_code) = f.await;
            log_trace!("recv complete: {} bytes, completion_code={}", len, completion_code);
            len - unused_len as usize
        })
    }
}
impl host::BulkEndpointOut for BulkOut {
    fn send<'a>(&'a self, buffer: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_debug!("send({}:{} {:?})", self.addr, self.index, ::kernel::logging::HexDump(buffer));
        {
            let mut state = self.host.push_ep_trbs(self.addr, self.index);
            
            // TODO: Try to use inline? Probably not worth it on bulk
            for (paddr, len, is_last) in super::iter_contigious_phys(buffer) {
                // SAFE: Trusting ourselves to wait until the hardware is done
                unsafe {
                    state.push(get_data(false, hw_structs::TrbNormalData::Pointer(paddr), len as u32, is_last));
                }
            }
        }

        let len = buffer.len();
        let f = self.host.wait_for_completion(self.addr, self.index);
        super::make_asyncwaitio(async move {
            let (unused_len, completion_code) = f.await;
            log_trace!("send complete: {} bytes, completion_code={}", len, completion_code);
            len - unused_len as usize
        })
    }
}