use ::usb_core::host;

/// Device0 - A special ControlEndpoint for device ID0 (uninitialised device)
pub(crate) struct Device0 {
	host: crate::HostRef,
}
impl Device0 {
	pub fn new(host: crate::HostRef, _max_packet_size: usize) -> Self {
		Device0 { host }
	}
}

impl host::ControlEndpoint for Device0 {
	fn out_only<'a>(&'a self, setup_data: &'a [u8], _out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
		// Request type 0, request number 5
		if setup_data.len() >= 4 && &setup_data[..2] == &[0x00, 5] {
			assert!(setup_data[3] == 0, "Setup data: {:?}", setup_data);
			let addr = setup_data[2];   // USB is little-endian!

			let f = self.host.set_address(addr);
			super::make_asyncwaitio(async move { f.await.expect("setting device address, TODO handle"); 0 })
		}
		else {
			panic!("Device::out_only: Only a SET_ADDRESS is valid");
		}
	}
	fn in_only<'a>(&'a self, _setup_data: &'a [u8], _out_data: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
		panic!("in_only on Device0 - not valid");
	}
}