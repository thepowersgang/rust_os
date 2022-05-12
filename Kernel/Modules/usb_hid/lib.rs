// "Tifflin" Kernel - USB HID driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_hid/lib.rs
//! USB HID (Human Interface Device) driver
#![no_std]
#![feature(linkage)]	// for module_define!
use kernel::prelude::*;

#[macro_use]
extern crate kernel;
extern crate usb_core;
extern crate gui;

mod report_parser;
// Sinks - Destinations for the inputs from a device
mod sinks;

module_define!{usb_hid, [usb_core, GUI], init}

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
		if class_code & 0x03_00_00 == 0x03_00_00 {
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

impl Driver
{
	/// Start the device worker
	async fn start_device_inner(ep0: &::usb_core::ControlEndpoint, endpoints: Vec<::usb_core::Endpoint>, report_desc_len: u16)
	{
		// 1. Request that descriptor from the device
		let mut report_spec = vec![0; report_desc_len as usize];
		let res_len = ep0.read_descriptor_raw(0x1000 | 0x22, 0, &mut report_spec).await.unwrap();
		assert!(res_len == report_spec.len(), "Report descriptor size mismatch");

		// 2. Parse the report descriptor, and locate collections of known usage
		// - Use collections to determine what bindings to set up
		let mut sinks = sinks::Group::from_report_spec(&report_spec);

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
			let mut bs = BitStream::new(&d);

			// Decode input using the report descriptor
			let mut state = report_parser::ParseState::default();
			for (id, val) in report_parser::IterRaw(&report_spec)
			{
				let op = report_parser::Op::from_pair(id, val);
				match op
				{
				report_parser::Op::Input(flags) => {
					for i in 0 .. state.report_count as usize
					{
						// If the input is an array, then the value gives you the usage
						let val = bs.get_i32(state.report_size as usize).unwrap_or(0);
						let usage = state.usage.get(if flags.is_variable() { i } else { val as usize });
						match usage
						{
						// Keyboard
						0x7_0000 ..= 0x7_00FF => {
							log_debug!("{:x} (key) = {}", usage, (val != 0));
							if val != 0 {
								sinks.keyboard.as_mut().unwrap().set_key( (usage & 0xFF) as u8 );
							}
							},
						// Mouse coords (relative or absolute)
						// "Generic Desktop" "X"/"Y"
						0x1_0030 ..= 0x1_0031 => {
							let is_x = usage & 1 == 0;
							let n = if is_x { "X" } else { "Y" };
							let mouse_sink = sinks.mouse.as_mut().unwrap();
							if flags.is_relative() {
								log_debug!("{:x} d{} = {}", usage, n, val);
								if is_x {
									mouse_sink.rel_x(val as i16);
								}
								else {
									mouse_sink.rel_y(val as i16);
								}
							}
							else {
								// Normalise into `0 ..= 0xFFFF`
								let lmin = state.logical_range.0.unwrap_or(0) as i32;
								let lmax = state.logical_range.1.unwrap_or(lmin + 1) as i32;
								let norm = (((val - lmin) as u64 * 0xFFFF) / (lmax - lmin) as u64) as u16;
								log_debug!("{:x} {} = {:#x} (raw = {:#x})", usage, n, norm, val);
								if is_x {
									mouse_sink.abs_x(norm);
								}
								else {
									mouse_sink.abs_y(norm);
								}
							}
							},
						// Scroll wheel
						0x1_0038 => {
							log_debug!("{:?} Scroll = {}", usage, val);
							},
						// Buttons (are these just mouse?)
						0x9_0001 ..= 0x9_0005 => {
							let num = (usage - 0x9_0001) as usize;
							log_debug!("{:x} Button {} = {}", usage, num, val);
							sinks.mouse.as_mut().unwrap().set_button(num-1, val != 0);
							},
						_ => {
							log_debug!("{:x} +{} ={:x}", usage, state.report_size, val);
							},
						}
					}
					},
				_ => {},
				}
				state.update(op);
			}

			if let Some(ref mut k) = sinks.keyboard {
				k.updated();
			}
			if let Some(ref mut s) = sinks.mouse {
				s.updated();
			}
		}
	}
}

struct BitStream<'a>(&'a [u8], usize);
impl<'a> BitStream<'a>
{
	fn new(d: &[u8]) -> BitStream {
		BitStream(d, 0)
	}
	fn get_bit(&mut self) -> Option<bool> {
		if self.0.len() == 0 {
			None
		}
		else {
			let rv = (self.0[0] >> self.1) & 1;
			self.1 += 1;
			if self.1 == 8 {
				self.0 = &self.0[1..];
				self.1 = 0;
			}
			Some( rv == 1 )
		}
	}
	fn get_u32_expensive(&mut self, bits: usize) -> Option<u32> {
		let mut rv = 0;
		for i in 0 .. bits {
			if self.get_bit()? {
				rv |= 1 << i;
			}
		}
		Some(rv)
	}
	fn get_u32(&mut self, bits: usize) -> Option<u32> {
		if self.0.len() == 0 {
			None
		}
		else if self.1 == 0 {
			if bits == 8 {
				let rv = self.0[0];
				self.0 = &self.0[1..];
				Some(rv as u32)
			}
			else if bits == 16 {
				let rv = self.0[0] as u32 | (*self.0.get(1)? as u32) << 8;
				self.0 = &self.0[2..];
				Some(rv)
			}
			else if bits < 8 {
				let rv = self.0[0] & ((1 << bits) - 1);
				self.1 += bits;
				Some(rv as u32)
			}
			else {
				self.get_u32_expensive(bits)
			}
		}
		else {
			self.get_u32_expensive(bits)
		}
	}
	fn get_i32(&mut self, bits: usize) -> Option<i32> {
		let mut u = self.get_u32(bits)?;
		let sgn_bit = 1 << (bits-1);
		if u & sgn_bit != 0 {
			u |= !(sgn_bit - 1);
		}
		Some(u as i32)
	}
}

