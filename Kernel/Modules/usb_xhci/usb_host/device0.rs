use ::usb_core::host;

pub(crate) struct Device0 {
    host: crate::HostRef,
}
impl Device0 {
    pub fn new(host: crate::HostRef, max_packet_size: usize) -> Self {
        Device0 { host }
    }
}

impl host::ControlEndpoint for Device0 {
    fn out_only<'a>(&'a self, setup_data: &'a [u8], _out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        // Request type 0, request number 5
        if setup_data.len() >= 4 && &setup_data[..2] == &[0x00, 5] {
            assert!(setup_data[3] == 0, "Setup data: {:?}", setup_data);
            let addr = setup_data[2];   // USB is little-endian!

            // Propagate the information currently assigned to Dev0 to the new device ID
            //self.host.set_usb1(addr, self.host.get_usb1(0));
            self.host.set_address(addr);

            host::AsyncWaitIo::new(async { 0 }).ok().expect("Should fit inline")
        }
        else {
            todo!("")
        }
    }
    fn in_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        todo!("");
    }
}