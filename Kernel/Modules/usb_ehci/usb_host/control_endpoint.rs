//!
//! 
use ::usb_core::host::{self,EndpointAddr};

pub struct ControlEndpoint
{
    host: crate::HostRef,
    endpoint: EndpointAddr,
    qh: Option<::kernel::futures::Mutex<crate::HostHeldQh>>,
}
impl ControlEndpoint
{
    pub(super) fn new(host: crate::HostRef, endpoint: EndpointAddr, max_packet_size: usize) -> Self {
        let usb1 = host.get_usb1(endpoint.dev_addr());
        let (endpoint_id, endpoint_ext) = super::make_endpoint_spec(endpoint, max_packet_size, usb1);
        let qh = host.qh_pool.alloc(endpoint_id, endpoint_ext);
        let qh = host.add_qh_to_async(qh);
        Self {
            host,
            endpoint,
            qh: Some(::kernel::futures::Mutex::new(qh)),
        }
    }

    pub fn get_dev_addr(&self) -> u8 {
        self.endpoint.dev_addr()
    }

    /// Helper function to ensure that the USB1 state of Dev0/Ep0 is always updated
    async fn get_qh(&self) -> ::kernel::futures::mutex::HeldMutex<'_, crate::HostHeldQh> {
        let mut qh = self.qh.as_ref().unwrap().async_lock().await;

        // If this is device 0 (inherently endpoint 0), then update the endpoint info
        if self.get_dev_addr() == 0 {
            let usb1 = self.host.get_usb1(0);
            let (endpoint_id, endpoint_ext) = self.host.edit_endpoint(&mut qh);
            super::set_usb1_state(endpoint_id, endpoint_ext, usb1);
        }

        qh
    }
}
impl host::ControlEndpoint for ControlEndpoint
{
	fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_debug!("ControlEndpoint::out_only({:?}): setup={:?} {}", self.endpoint, ::kernel::logging::HexDump(setup_data), out_data.len());
        // Note: reverse order to set up the chaining
        // Get a TD for the status (PID_IN)
        let td_status = self.host.td_pool.alloc(crate::hw_structs::Pid::In, &[], None);
        // Get a TD for the output (PID_OUT) - Optional
        let td_data = if out_data.len() > 0 {
                self.host.td_pool.alloc(crate::hw_structs::Pid::Out, out_data, Some(td_status))
            }
            else {
                td_status
            };
        // Get a TD for the setup (PID_SETUP)
        let td_setup = self.host.td_pool.alloc(crate::hw_structs::Pid::Setup, setup_data, Some(td_data));
        
        super::make_asyncwaitio(async move {
            let mut qh = self.get_qh().await;
            let td_setup = self.host.wait_for_async(&mut qh, td_setup).await;
            let mut td_data = self.host.td_pool.release(td_setup).unwrap();
            
            let unused_len = (self.host.td_pool.get_data(&mut td_data).token >> 16) & 0x7FFF;

            let td_status = self.host.td_pool.release(td_data);
            if let Some(td_status) = td_status {
                assert!(self.host.td_pool.release(td_status).is_none());
            }

            // - If this endpoint is dev0/ep0, then look for an address set request
            if self.get_dev_addr() == 0
            {
                // Request type 0, request number 5
                if setup_data.len() >= 4 && &setup_data[..2] == &[0x00, 5] {
                    assert!(setup_data[3] == 0, "Setup data: {:?}", setup_data);
                    let addr = setup_data[2];   // USB is little-endian!
                    // Propagate the information currently assigned to Dev0 to the new device ID
                    self.host.set_usb1(addr, self.host.get_usb1(0));
                    // NOTE: The new device ID won't be used until this function returns, so we're good
                }
            }

            out_data.len() - unused_len as usize
        })
    }
	fn in_only<'a>(&'a self, setup_data: &'a [u8], in_buf: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        log_debug!("ControlEndpoint::in_only({:?}): setup={:?} {}", self.endpoint, ::kernel::logging::HexDump(setup_data), in_buf.len());
        // Note: reverse order to set up the chaining
        // Get a TD for the status (PID_IN)
        let td_status = self.host.td_pool.alloc(crate::hw_structs::Pid::Out, &[], None);
        // Get a TD for the output (PID_OUT)
        let td_data = if in_buf.len() > 0 {
                self.host.td_pool.alloc(crate::hw_structs::Pid::In, in_buf, Some(td_status))
            }
            else {
                td_status
            };
        // Get a TD for the setup (PID_SETUP)
        let td_setup = self.host.td_pool.alloc(crate::hw_structs::Pid::Setup, setup_data, Some(td_data));
        
        super::make_asyncwaitio(async move {
            let mut qh = self.get_qh().await;

            let td_setup = self.host.wait_for_async(&mut qh, td_setup).await;
            let mut td_data = self.host.td_pool.release(td_setup).unwrap();
            
            let unused_len = (self.host.td_pool.get_data(&mut td_data).token >> 16) & 0x7FFF;
            let td_status = self.host.td_pool.release(td_data);
            if let Some(td_status) = td_status {
                assert!(self.host.td_pool.release(td_status).is_none());
            }

            in_buf.len() - unused_len as usize
        })
    }
}

impl ::core::ops::Drop for ControlEndpoint
{
    fn drop(&mut self) {
        self.host.remove_qh_from_async(self.qh.take().unwrap().into_inner());
    }
}