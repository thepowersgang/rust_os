// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/input/mod.rs
//! GUI input managment
#[allow(unused_imports)]
use kernel::prelude::*;
use self::keyboard::KeyCode;
use core::sync::atomic::{Ordering,AtomicUsize,AtomicU8};
use kernel::sync::Mutex;

pub mod keyboard;
pub mod mouse;

#[derive(Debug)]
pub enum Event
{
	KeyDown(keyboard::KeyCode),
	KeyUp(keyboard::KeyCode),
	KeyFire(keyboard::KeyCode),
	Text([u8; 6]),	// 6 bytes, as that can fit in a u64 with a 16-bit tag

	MouseMove(u32,u32,i16,i16),
	MouseDown(u32,u32,u8),
	MouseUp(u32,u32,u8),
	MouseClick(u32,u32, u8, u8),
}

struct ModKeyPair(AtomicUsize);

struct MouseCursor {
	graphics_cursor: ::kernel::sync::Mutex<::kernel::metadevs::video::CursorHandle>,
}

struct InputChannel
{
	//caps_active: AtomicBool,	// Go DIAF capslock
	shift_held: ModKeyPair,
	ctrl_held: ModKeyPair,
	alt_held: ModKeyPair,
	//altgr: ModKeyPair,	// AltGr is usually just one... but meh
	
	last_key_pressed: AtomicU8,
	//active_repeat: AtomicValue<u8>,
	//repeat_start: Timestamp,
	
	cursor: MouseCursor,
	// TODO: Mutex feels too heavy, but there may be multiple mice on one channel
	double_click_info: Mutex<MouseClickInfo>,
}

struct MouseClickInfo
{
	button: u8,
	count: u8,
	time: ::kernel::time::TickCount,
	x: u32,
	y: u32,
}

//struct IMEState
//{
//	ime_ofs: u8,
//	ime_val: u32,
//}

/// Maximum time in kernel ticks between subsequent press/release events for a click/doubleclick
const DOUBLE_CLICK_TIMEOUT: u64 = 500;	// 500ms
/// Maximum distance along any axis between press/release before a click is not registered
const MAX_CLICK_MOVE: u32 = 10;
static MAIN_INPUT: InputChannel = InputChannel::new();

pub fn init() {
	//MAIN_INPUT.cursor.
}

fn get_channel_by_index(_idx: usize) -> &'static InputChannel {
	&MAIN_INPUT
}

impl InputChannel
{
	const fn new() -> InputChannel {
		InputChannel { 
			shift_held: ModKeyPair::new(),
			ctrl_held: ModKeyPair::new(),
			alt_held: ModKeyPair::new(),
			//altgr: ModKeyPair::new(),
			cursor: MouseCursor::new(),
			
			last_key_pressed: AtomicU8::new(KeyCode::None as u8),
			double_click_info: Mutex::new(MouseClickInfo::new()),
			}
	}
	pub fn handle_key(&self, key: keyboard::KeyCode, release: bool)
	{
		log_trace!("key={:?}, release={}", key, release);
		match (release, key)
		{
		// Maintain key states
		(false, KeyCode::RightShift) => self.shift_held.set_r(),
		(false, KeyCode::LeftShift)  => self.shift_held.set_l(),
		(false, KeyCode::RightCtrl) => self.ctrl_held.set_r(),
		(false, KeyCode::LeftCtrl)  => self.ctrl_held.set_l(),
		(false, KeyCode::RightAlt) => self.alt_held.set_r(),
		(false, KeyCode::LeftAlt)  => self.alt_held.set_l(),
		(true, KeyCode::RightShift) => self.shift_held.clear_r(),
		(true, KeyCode::LeftShift)  => self.shift_held.clear_l(),
		(true, KeyCode::RightCtrl) => self.ctrl_held.clear_r(),
		(true, KeyCode::LeftCtrl)  => self.ctrl_held.clear_l(),
		(true, KeyCode::RightAlt) => self.alt_held.clear_r(),
		(true, KeyCode::LeftAlt)  => self.alt_held.clear_l(),
		// Check for session change commands, don't propagate if they fired
		// - 'try_change_session' checks for the required modifier keys and permissions
		// TODO: Should this be handled by the `windows` module?
		(false, KeyCode::Esc) => if self.try_change_session(0) { return ; },
		(false, KeyCode::F1)  => if self.try_change_session(1) { return ; },
		(false, KeyCode::F2)  => if self.try_change_session(2) { return ; },
		(false, KeyCode::F3)  => if self.try_change_session(3) { return ; },
		(false, KeyCode::F4)  => if self.try_change_session(4) { return ; },
		(false, KeyCode::F5)  => if self.try_change_session(5) { return ; },
		(false, KeyCode::F6)  => if self.try_change_session(6) { return ; },
		(false, KeyCode::F7)  => if self.try_change_session(7) { return ; },
		(false, KeyCode::F8)  => if self.try_change_session(8) { return ; },
		(false, KeyCode::F9)  => if self.try_change_session(9) { return ; },
		(false, KeyCode::F10) => if self.try_change_session(10) { return ; },
		(false, KeyCode::F11) => if self.try_change_session(11) { return ; },
		(false, KeyCode::F12) => if self.try_change_session(12) { return ; },
		_ => {},
		}

		let last_key = self.last_key_pressed.load(Ordering::Relaxed);
		if !release {
			self.last_key_pressed.store(key as u8, Ordering::Relaxed);
			super::windows::handle_input(/*self, */Event::KeyDown(key));
		}

		// Handle fire and text events
		if key.is_modifier()
		{
			// Only fire a modifier on key-up IF they were the last one pressed
			// - This allows "Gui" (windows) to fire on key-up while still being used as a modifier
			if release && last_key == key as u8
			{
				super::windows::handle_input( Event::KeyFire(key) );
			}
		}
		else
		{
			// TODO: Support repetition (of the last non-modifier pressed)
			if !release
			{
				super::windows::handle_input( Event::KeyFire(key) );

				// TODO: Should only generate text if no non-shift modifiers are depressed
				//if self.enable_input_translation {
					let s = self.get_input_string(key);
					if s.len() > 0 {
						let mut buf = [0; 6];
						buf[.. s.len()].clone_from_slice( s.as_bytes() );
						super::windows::handle_input( Event::Text(buf) );
					}
				//}
			}
		}

		// Send key combination to active active window (via the window subsystem)
		if release {
			self.last_key_pressed.store(KeyCode::None as u8, Ordering::Relaxed);
			super::windows::handle_input(/*self, */Event::KeyUp(key));
		}
	}
	
	pub fn handle_mouse_set(&self, norm_x: u16, norm_y: u16)
	{
		// Mouse movement, update cursor
		let (dx,dy) = self.cursor.set_pos(norm_x, norm_y);
		let (x,y) = self.cursor.pos();
		self.double_click_info.lock().clear();
		super::windows::handle_input(/*self, */Event::MouseMove(x, y, dx as i16, dy as i16));
	}
	pub fn handle_mouse_move(&self, dx: i16, dy: i16)
	{
		// Mouse movement, update cursor
		self.cursor.move_pos(dx as i32, dy as i32);
		let (x,y) = self.cursor.pos();
		self.double_click_info.lock().clear();
		super::windows::handle_input(/*self, */Event::MouseMove(x, y, dx, dy));
	}
	pub fn handle_mouse_btn(&self, btn: u8, release: bool)
	{
		let (x,y) = self.cursor.pos();
		if release
		{

			// Released - check the double-click timer
			if let Some(ev) = self.double_click_info.lock().check( x,y, btn )
			{
				super::windows::handle_input(/*self, */ev);
			}

			super::windows::handle_input(/*self, */Event::MouseUp(x, y, btn));
		}
		else
		{

			// Pressed - reset the double-click timer
			self.double_click_info.lock().reset(x,y, btn);

			super::windows::handle_input(/*self, */Event::MouseDown(x, y, btn));
		}
	}

	fn shift(&self) -> bool {
		self.shift_held.get()
	}
	fn upper(&self) -> bool {
		self.shift()
	}
	
	fn get_input_string(&self, keycode: KeyCode) -> &str
	{
		macro_rules! shift { ($s:ident: $lower:expr, $upper:expr) => { if $s.shift() { $upper } else {$lower} }; }
		macro_rules! alpha { ($s:ident: $lower:expr, $upper:expr) => { if $s.upper() { $upper } else {$lower} }; }
		match keycode
		{
		KeyCode::A => alpha!(self: "a", "A"),
		KeyCode::B => alpha!(self: "b", "B"),
		KeyCode::C => alpha!(self: "c", "C"),
		KeyCode::D => alpha!(self: "d", "D"),
		KeyCode::E => alpha!(self: "e", "E"),
		KeyCode::F => alpha!(self: "f", "F"),
		KeyCode::G => alpha!(self: "g", "G"),
		KeyCode::H => alpha!(self: "h", "H"),
		KeyCode::I => alpha!(self: "i", "I"),
		KeyCode::J => alpha!(self: "j", "J"),
		KeyCode::K => alpha!(self: "k", "K"),
		KeyCode::L => alpha!(self: "l", "L"),
		KeyCode::M => alpha!(self: "m", "M"),
		KeyCode::N => alpha!(self: "n", "N"),
		KeyCode::O => alpha!(self: "o", "O"),
		KeyCode::P => alpha!(self: "p", "P"),
		KeyCode::Q => alpha!(self: "q", "Q"),
		KeyCode::R => alpha!(self: "r", "R"),
		KeyCode::S => alpha!(self: "s", "S"),
		KeyCode::T => alpha!(self: "t", "T"),
		KeyCode::U => alpha!(self: "u", "U"),
		KeyCode::V => alpha!(self: "v", "V"),
		KeyCode::W => alpha!(self: "w", "W"),
		KeyCode::X => alpha!(self: "x", "X"),
		KeyCode::Y => alpha!(self: "y", "Y"),
		KeyCode::Z => alpha!(self: "z", "Z"),

		KeyCode::SquareOpen  => shift!(self: "[", "{"),
		KeyCode::SquareClose => shift!(self: "[", "{"),
		KeyCode::Backslash   => shift!(self: "\\","|"),
		KeyCode::Semicolon => shift!(self: ";", ":"),
		KeyCode::Quote     => shift!(self: "'", "\""),
		KeyCode::Comma  => shift!(self: ",", "<"),
		KeyCode::Period => shift!(self: ".", ">"),
		KeyCode::Slash  => shift!(self: "/", "?"),

		KeyCode::Kb1 => shift!(self: "1", "!"),
		KeyCode::Kb2 => shift!(self: "2", "@"),
		KeyCode::Kb3 => shift!(self: "3", "#"),
		KeyCode::Kb4 => shift!(self: "4", "$"),
		KeyCode::Kb5 => shift!(self: "5", "%"),
		KeyCode::Kb6 => shift!(self: "6", "^"),
		KeyCode::Kb7 => shift!(self: "7", "&"),
		KeyCode::Kb8 => shift!(self: "8", "*"),
		KeyCode::Kb9 => shift!(self: "9", "("),
		KeyCode::Kb0 => shift!(self: "0", ")"),
		KeyCode::Minus  => shift!(self: "-", "_"),
		KeyCode::Equals => shift!(self: "=", "+"),

		KeyCode::Space => " ",
		_ => "",
		}
	}
	
	fn try_change_session(&self, target: usize) -> bool {
		if self.is_master() && self.ctrl_held.get() && self.alt_held.get() {
			super::windows::switch_active(target);
			true
		}
		else {
			false
		}
	}
	
	fn is_master(&self) -> bool { true }
}

impl ModKeyPair {
	const fn new() -> ModKeyPair {
		ModKeyPair(AtomicUsize::new(0))
	}
	fn set_l(&self) { self.0.fetch_or(1, Ordering::Relaxed); }
	fn set_r(&self) { self.0.fetch_or(2, Ordering::Relaxed); }
	fn clear_l(&self) { self.0.fetch_and(!1, Ordering::Relaxed); }
	fn clear_r(&self) { self.0.fetch_and(!2, Ordering::Relaxed); }
	fn get(&self) -> bool {
		self.0.load(Ordering::Relaxed) != 0
	}
}
impl MouseCursor {
	const fn new() -> MouseCursor {
		MouseCursor {
			graphics_cursor: ::kernel::sync::Mutex::new(::kernel::metadevs::video::CursorHandle::new()),
			}
	}
	fn add_coord(cur: u32, d: i32) -> u32 {
		if d < 0 {
			u32::saturating_sub(cur, -d as u32)
		}
		else {
			u32::saturating_add(cur, d as u32)
		}
	}

	/// Set cursor position to normalised coordinates
	fn set_pos(&self, norm_x: u16, norm_y: u16) -> (i32, i32) {
		let mut lh = self.graphics_cursor.lock();
		let pos = lh.get_pos();
		let rect = match ::kernel::metadevs::video::get_display_for_pos(pos)
			{
			Ok(v) => v,
			Err(v) => v,
			};
		let new_pos = ::kernel::metadevs::video::Pos {
			x: rect.x() + ((rect.w() as u64 * norm_x as u64) >> 16) as u32,
			y: rect.y() + ((rect.h() as u64 * norm_y as u64) >> 16) as u32,
			};
		lh.set_pos(new_pos);
		(
			(new_pos.x as i32 - pos.x as i32) as i32,
			(new_pos.y as i32 - pos.y as i32) as i32,
			)
	}
	fn move_pos(&self, dx: i32, dy: i32) {
		let mut lh = self.graphics_cursor.lock();
		let mut pos = lh.get_pos();
		pos.x = Self::add_coord(pos.x, dx);
		pos.y = Self::add_coord(pos.y, dy);
		lh.set_pos(pos);
	}
	fn pos(&self) -> (u32,u32) {
		let pos = self.graphics_cursor.lock().get_pos();
		(pos.x, pos.y)
	}
}

impl MouseClickInfo
{
	const fn new() -> MouseClickInfo {
		MouseClickInfo {
			button: 0xFF, x: 0, y: 0,
			time: 0,
			count: 0,
			}
	}
	fn clear(&mut self)
	{
		self.button = 0xFF;
	}
	fn reset(&mut self, x: u32, y: u32, button: u8)
	{
		self.button = button;
		self.count = 0;
		self.x = x;
		self.y = y;
		self.time = ::kernel::time::ticks();
	}

	fn check(&mut self, x: u32, y: u32, button: u8) -> Option<Event>
	{
		use kernel::lib::num::abs_diff;
		if self.button != button {
			self.clear();
			None
		}
		else if (::kernel::time::ticks() - self.time) > DOUBLE_CLICK_TIMEOUT {
			self.clear();
			None
		}
		else if abs_diff(self.x, x) > MAX_CLICK_MOVE || abs_diff(self.y, y) > MAX_CLICK_MOVE {
			self.clear();
			None
		}
		else {
			self.time = ::kernel::time::ticks();
			self.x = x;
			self.y = y;
			if self.count < 0xFF {
				self.count += 1;
			}

			Some( Event::MouseClick(x, y, button, self.count) )
		}
	}
}
