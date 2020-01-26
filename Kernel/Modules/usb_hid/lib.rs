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

		fn input(&self, _sinks: &mut super::sinks::Group, _state: &ParseState, _bits: crate::report_parser::InputFlags) { }
		fn output(&self, _sinks: &mut super::sinks::Group, _state: &ParseState, _bits: u32) { }
		fn feature(&self, _sinks: &mut super::sinks::Group, _state: &ParseState, _bits: u32) { }
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
				0x0001_0006 => Some(&Keyboard),	// "General Desktop" -> Keyboard
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
		fn input(&self, sinks: &mut super::sinks::Group, _state: &ParseState, bits: crate::report_parser::InputFlags) {
			log_debug!("Mouse Input {:?} ({:?})", bits, _state);
			if sinks.mouse.is_none() {
				sinks.mouse = Some(super::sinks::Mouse::new());
			}
			// TODO: determine if relative/absolute, and if scroll wheel is present (and button count?)
		}
	}

	struct Keyboard;
	impl Handler for Keyboard
	{
		fn parent(&self) -> &'static dyn Handler { &Root }
		fn child(&self, _state: &ParseState, _num: u32) -> Option<&'static dyn Handler> {
			None
		}
		fn input(&self, sinks: &mut super::sinks::Group, _state: &ParseState, bits: crate::report_parser::InputFlags) {
			log_debug!("Keyboard Input {:?} ({:?})", bits, _state);
			if sinks.keyboard.is_none() {
				sinks.keyboard = Some(super::sinks::Keyboard::new());
			}
		}
	}
}

mod sinks
{
	#[derive(Default)]
	pub struct Group
	{
		pub keyboard: Option<Keyboard>,
		pub mouse: Option<Mouse>,
	}
	pub struct Keyboard
	{
		cur_state: BitSet256,
		last_state: BitSet256,
		gui_handle: ::gui::input::keyboard::Instance,
	}
	impl Keyboard
	{
		pub fn new() -> Self {
			Keyboard {
				cur_state: BitSet256::new(),
				last_state: BitSet256::new(),
				gui_handle: ::gui::input::keyboard::Instance::new(),
				}
		}
		pub fn set_key(&mut self, k: u8)
		{
			self.cur_state.set( k as usize );
		}
		pub fn updated(&mut self) {
			for i in 0 .. 256
			{
				let cur = self.cur_state.get(i);
				let prev = self.last_state.get(i);

				if cur != prev
				{
					let k = match ::gui::input::keyboard::KeyCode::try_from( i as u8 )
						{
						Some(k) => k,
						None => {
							log_notice!("Bad key code: {:02x}", i);
							continue
							},
						};

					if cur {
						self.gui_handle.press_key(k);
					}
					else {
						self.gui_handle.release_key(k);
					}
				}
			}
			self.last_state = ::core::mem::replace(&mut self.cur_state, BitSet256::new());
		}
	}
	struct BitSet256([u8; 256/8]);
	#[allow(dead_code)]
	impl BitSet256
	{
		pub fn new() -> Self {
			BitSet256([0; 256/8])
		}
		pub fn get(&self, i: usize) -> bool {
			if i >= 256 {
				return false;
			}
			self.0[i / 8] & 1 << (i%8) != 0
		}
		pub fn set(&mut self, i: usize) {
			if i < 256 {
				self.0[i / 8] |= 1 << (i%8);
			}
		}
		pub fn clr(&mut self, i: usize) {
			if i < 256 {
				self.0[i / 8] &= !(1 << (i%8));
			}
		}
	}
	impl ::core::ops::BitXor for &'_ BitSet256
	{
		type Output = BitSet256;
		fn bitxor(self, other: &BitSet256) -> BitSet256
		{
			let mut rv = BitSet256::new();
			for (d,(a,b)) in Iterator::zip( rv.0.iter_mut(), Iterator::zip(self.0.iter(), other.0.iter()) )
			{
				*d = *a ^ *b;
			}
			rv
		}
	}

	pub struct Mouse
	{
		// Store x/y values (relative/absolute)
		is_relative: bool,
		x_value: u16,
		y_value: u16,

		// Button states
		cur_buttons: u16,
		prev_buttons: u16,

		gui_handle: ::gui::input::mouse::Instance,
	}
	impl Mouse
	{
		pub fn new() -> Mouse
		{
			Mouse {
				is_relative: false,
				x_value: 0, y_value: 0,
				cur_buttons: 0,
				prev_buttons: 0,
				gui_handle: ::gui::input::mouse::Instance::new(),
				}
		}

		pub fn abs_x(&mut self, v: u16) {
			self.x_value = v;
			self.is_relative = false;
		}
		pub fn abs_y(&mut self, v: u16) {
			self.y_value = v;
			self.is_relative = false;
		}
		pub fn rel_x(&mut self, d: i16) {
			self.x_value = d as u16;
			self.is_relative = true;
		}
		pub fn rel_y(&mut self, d: i16) {
			self.y_value = d as u16;
			self.is_relative = true;
		}

		pub fn set_button(&mut self, idx: usize, is_pressed: bool) {
			if is_pressed && idx < 16 {
				self.cur_buttons |= 1 << idx;
			}
		}

		pub fn updated(&mut self) {
			// Update positions
			if self.is_relative {
				self.gui_handle.move_cursor(self.x_value as i16, self.y_value as i16);
			}
			else {
				// NOTE: Should already be normalised
				self.gui_handle.set_cursor(self.x_value, self.y_value);
			}
			// Update buttons
			for i in 0 .. 16
			{
				let cur  = (self.cur_buttons  & 1 << i) != 0;
				let prev = (self.prev_buttons & 1 << i) != 0;

				if cur != prev
				{
					if cur {
						self.gui_handle.press_button(i);
					}
					else {
						self.gui_handle.release_button(i);
					}
				}
			}
			self.prev_buttons = self.cur_buttons;
			self.cur_buttons = 0;
		}
	}
	
}

impl Driver
{
	fn prepare_sinks(buf: &[u8]) -> sinks::Group {
		let mut sinks = sinks::Group::default();

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
					collection.input(&mut sinks, &state, v);
				}
				else {
					log_debug!("> INPUT {:?} {:?}", v, state);
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

		sinks
	}

	async fn start_device_inner(ep0: &::usb_core::ControlEndpoint, endpoints: Vec<::usb_core::Endpoint>, report_desc_len: u16)
	{
		// 1. Request that descriptor from the device
		let mut buf = vec![0; report_desc_len as usize];
		let res_len = ep0.read_descriptor_raw(0x1000 | 0x22, 0, &mut buf).await.unwrap();
		assert!(res_len == buf.len(), "Report descriptor size mismatch");

		// 2. Parse the report descriptor, and locate collections of known usage
		// - Use collections to determine what bindings to set up
		let mut sinks = Self::prepare_sinks(&buf);

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
			for (id, val) in report_parser::IterRaw(&buf)
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
						// Scroll wheel? Pressure?
						//0x1_0038 => {
						//	},
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

