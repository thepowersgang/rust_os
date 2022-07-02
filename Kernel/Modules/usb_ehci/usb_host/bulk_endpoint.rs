
use ::usb_core::host::{self,EndpointAddr};

pub struct BulkEndpoint
{
    host: crate::HostRef,
    endpoint: EndpointAddr,
    qh: Option<::kernel::futures::Mutex<crate::HostHeldQh>>,
}

impl BulkEndpoint
{
    pub(super) fn new(host: crate::HostRef, endpoint: EndpointAddr, max_packet_size: usize) -> Self {
        let usb1 = host.get_usb1(endpoint.dev_addr());
        let (endpoint_id, endpoint_ext) = super::make_endpoint_spec(endpoint, max_packet_size, usb1, false);
        let qh = host.qh_pool.alloc(endpoint_id, endpoint_ext);
        let qh = host.add_qh_to_async(qh);
        Self {
            host,
            endpoint,
            qh: Some(::kernel::futures::Mutex::new(qh)),
        }
    }
}

impl host::BulkEndpointOut for BulkEndpoint
{
	fn send<'a>(&'a self, buffer: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_debug!("send({:?}): buffer={:?}", self.endpoint, ::kernel::logging::HexDump(buffer));

        // SAFE:? Could read freed data if the future is cancelled (minimal risk)
        let td_data = unsafe { self.host.td_pool.alloc(crate::hw_structs::Pid::Out, buffer, None) };
        
        super::make_asyncwaitio(async move {
            let mut qh = self.qh.as_ref().unwrap().async_lock().await;
            let mut td_data = self.host.wait_for_async(&mut qh, td_data).await;
            
            let unused_len = (self.host.td_pool.get_data(&mut td_data).token >> 16) & 0x7FFF;

            assert!(self.host.td_pool.release(td_data).is_none());

            buffer.len() - unused_len as usize
        })
    }
}

impl host::BulkEndpointIn for BulkEndpoint
{
	fn recv<'a>(&'a self, buffer: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_debug!("recv({:?}): buffer={} b", self.endpoint, buffer.len());

        // SAFE:? Could write to a freed buffer if the future is cancelled (that'd be bad)
        let td_data = unsafe { self.host.td_pool.alloc(crate::hw_structs::Pid::In, buffer, None) };
        
        super::make_asyncwaitio(async move {
            let mut qh = self.qh.as_ref().unwrap().async_lock().await;
            let mut td_data = self.host.wait_for_async(&mut qh, td_data).await;
            
            let unused_len = (self.host.td_pool.get_data(&mut td_data).token >> 16) & 0x7FFF;

            assert!(self.host.td_pool.release(td_data).is_none());

            buffer.len() - unused_len as usize
        })
    }
}

impl ::core::ops::Drop for BulkEndpoint
{
    fn drop(&mut self) {
        self.host.remove_qh_from_async(self.qh.take().unwrap().into_inner());
    }
}