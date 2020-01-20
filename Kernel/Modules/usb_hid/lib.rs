// 
//
//
//! USB HID (Human Interface Device) driver
#![no_std]
#![feature(linkage)]	// for module_define!
use kernel::prelude::*;

#[macro_use]
extern crate kernel;
extern crate usb_core;

mod report_parser;

module_define!{usb_hid, [usb_core], init}

fn init()
{
	static USB_DRIVER: Driver = Driver;
	::usb_core::device::register_driver(&USB_DRIVER);
}

struct Driver;
impl ::usb_core::device::Driver for Driver
{
	fn name(&self) -> &str {
		"hid"
	}
	fn matches(&self, _vendor_id: u16, _device_id: u16, class_code: u32) -> ::usb_core::device::MatchLevel {
		use ::usb_core::device::MatchLevel;
		if class_code == 0x03_00_00 {
			MatchLevel::Generic
		}
		else {
			MatchLevel::None
		}
	}
	fn start_device<'a>(&self, ep0: &'a ::usb_core::ControlEndpoint, endpoints: Vec<::usb_core::Endpoint>, descriptors: &[u8]) -> ::usb_core::device::Instance<'a> {
		// 1. Find the HID descriptor in the list
		// 2. Locate the report descriptor (0x22) and get the length
		let mut report_desc_len = 0;
		for d in ::usb_core::hw_decls::IterDescriptors(descriptors)
		{
			// 0x21 = HID Descriptor
			if d[1] == 0x21
			{
				// TODO: Get the header
				let ofs = 6;
				let len = d[0] - ofs;
				if len % 3 != 0 {
					log_error!("Invalid HID descriptor: bad length");
					continue ;
				}
				for sd in d[6..].chunks(3)
				{
					let ty = sd[0];
					let len = sd[1] as u16 | (sd[2] as u16) << 8;
					//log_debug!("USB HID Desc {:02x} len={}", ty, len);
					if ty == 0x22 {
						report_desc_len = len;
					}
				}
			}
		}
		// Hand off to the async code (which isn't borrowing the descriptor list)
		Box::new(Self::start_device_inner(ep0, endpoints, report_desc_len))
	}
}

mod collection_parse
{
	use crate::report_parser::ParseState;

	pub fn root() -> &'static dyn Handler {
		&Root
	}
	pub trait Handler
	{
		fn parent(&self) -> &'static dyn Handler;
		fn child(&self, state: &ParseState, num: u32) -> Option<&'static dyn Handler>;

		fn input(&self, _state: &ParseState, _bits: u32) { }
		fn output(&self, _state: &ParseState, _bits: u32) { }
		fn feature(&self, _state: &ParseState, _bits: u32) { }
	}
	struct Root;
	impl Handler for Root
	{
		fn parent(&self) -> &'static dyn Handler { &Root }
		fn child(&self, state: &ParseState, num: u32) -> Option<&'static dyn Handler> {
			match num
			{
			1 => match state.usage.get(0)
				{
				0x0001_0001 => None,	// "General Desktop" -> Pointer
				0x0001_0002 => Some(&Mouse),	// "General Desktop" -> Mouse
				0x0001_0004 => None,	// "General Desktop" -> Joystick
				0x0001_0005 => None,	// "General Desktop" -> Game Pad
				0x0001_0006 => None,	// "General Desktop" -> Keyboard
				0x0007_0000 ..= 0x0007_FFFF => None,	// Keyboard/Keypad
				_ => None,
				},
			_ => None,
			}
		}
	}

	struct Mouse;
	impl Handler for Mouse
	{
		fn parent(&self) -> &'static dyn Handler { &Root }
		fn child(&self, _state: &ParseState, _num: u32) -> Option<&'static dyn Handler> {
			Some(&MouseInner)
		}
	}
	struct MouseInner;
	impl Handler for MouseInner
	{
		fn parent(&self) -> &'static dyn Handler { &Mouse }
		fn child(&self, _state: &ParseState, _num: u32) -> Option<&'static dyn Handler> {
			None
		}
		fn input(&self, _state: &ParseState, bits: u32) {
			log_debug!("Mouse Input {:b} ({:?})", bits, _state);
		}
	}
}

impl Driver
{
	async fn start_device_inner(ep0: &::usb_core::ControlEndpoint, endpoints: Vec<::usb_core::Endpoint>, report_desc_len: u16)
	{
		// 1. Request that descriptor from the device
		let mut buf = vec![0; report_desc_len as usize];
		let res_len = ep0.read_descriptor_raw(0x1000 | 0x22, 0, &mut buf).await.unwrap();
		assert!(res_len == buf.len(), "Report descriptor size mismatch");

		// 2. Parse the report descriptor, and locate collections of known usage
		// - Use collections to determine what bindings to set up
		{
			let mut collection = collection_parse::root();
			let mut collection_depth = 0;
			let mut state = report_parser::ParseState::default();
			for (id, val) in report_parser::IterRaw(&buf)
			{
				let op = report_parser::Op::from_pair(id, val);
				log_debug!("> {:?}", op);
				match op
				{
				report_parser::Op::Collection(num) => {
					// Check the current collection state.
					match collection.child(&state, num)
					{
					// if this number is known, update current state
					Some(v) => collection = v,
					// else, increment depth
					None => collection_depth += 1,
					}
					},
				report_parser::Op::EndCollection => {
					// If depth is non-zero, decrement
					if collection_depth > 0 {
						collection_depth -= 1;
					}
					// else, go to current collection parent
					else {
						collection = collection.parent();
					}
					},
				report_parser::Op::Input(v) => {
					if collection_depth == 0 {
						collection.input(&state, v);
					}
					else {
						log_debug!("> INPUT {:09b} {:?}", v, state);
					}
					},
				report_parser::Op::Output(v) => {
					log_debug!("> OUTPUT {:09b} {:?}", v, state);
					},
				report_parser::Op::Feature(v) => {
					log_debug!("> FEATURE {:09b} {:?}", v, state);
					},
				_ => {},
				}
				state.update(op);
			}
		}

		let mut int_endpoint = None;
		for ep in endpoints
		{
			match ep
			{
			::usb_core::Endpoint::Interrupt(ep) => { int_endpoint = Some(ep); },
			_ => {},
			}
		}
		let int_endpoint = int_endpoint.expect("No interrupt endpoint on a HID device?");

		// 3. Start polling the interrupt endpoint
		// - Use the report descriptor to parse it
		loop
		{
			let d = int_endpoint.wait().await;
			//let mut bs = BitStream::new(&d);

			// Decode input using the report descriptor
			let mut state = report_parser::ParseState::default();
			for (id, val) in report_parser::IterRaw(&buf)
			{
				let op = report_parser::Op::from_pair(id, val);
				match op
				{
				report_parser::Op::Input(_v) =>
					for i in 0 .. state.report_count as usize
					{
						let usage = state.usage.get(i);
						log_debug!("{:x} +{}", usage, state.report_size);
					},
				_ => {},
				}
				state.update(op);
			}

		}
	}
}

