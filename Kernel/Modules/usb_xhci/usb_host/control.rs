use ::usb_core::host;
use ::kernel::lib::mem::Box;

pub struct Control
{
    host: crate::HostRef,
    addr: u8,
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
    pub(crate) fn new(host: crate::HostRef, addr: u8, endpoint: u8, max_packet_size: usize) -> Self {
        // Need to get an endpoint handle from the host.
        //host.claim_endpoint(addr, endpoint);
        Control { host, addr, endpoint }
    }
}

impl Endpoint0
{
    pub(crate) fn new(host: crate::HostRef, addr: u8, max_packet_size: usize) -> Self {
        Endpoint0 { inner: Control::new(host, addr, 0, max_packet_size) }
    }
}

impl host::ControlEndpoint for Control {
    fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        let idx = if self.endpoint == 0 { 1 } else { self.endpoint * 2 + 0 };
        // Create TRBs for the data (Setup, data, status)
        //self.host.push_ep_trbs(self.addr, idx, &[
        //    ]);
        // Add it to the TRB for this endpoint
        todo!("out_only")
    }
    fn in_only<'a>(&'a self, setup_data: &'a [u8], in_data: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        let idx = self.endpoint * 2 + 1;
        todo!("in_only")
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
            if setup_data[2] == 2 /* Descriptor_Configuration */ {
                todo!("Handle GET_DESCRIPTOR for configuration");
                // - Send the message, but intercept the reply
                // - Decode the configuration and count the number of endpoints
                // - Save endpoint count against the configuration index
                // - Inform the host of this endpoint count
            }
        }
        self.inner.in_only(setup_data, in_data)
    }
}