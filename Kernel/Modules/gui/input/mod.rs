// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/input/mod.rs
//! GUI input managment
#[allow(unused_imports)]
use kernel::prelude::*;
use self::keyboard::KeyCode;
use core::sync::atomic::{AtomicUsize,ATOMIC_USIZE_INIT,Ordering};

pub mod keyboard;
pub mod mouse;

#[derive(Debug)]
pub enum Event
{
	KeyDown(keyboard::KeyCode),
	KeyUp(keyboard::KeyCode),
	Text([u8; 6]),	// 6 bytes, as that can fit in a u64 with a 16-bit tag

	MouseMove(u32,u32,i16,i16),
	MouseDown(u32,u32,u8),
	MouseUp(u32,u32,u8),
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
	
	cursor: MouseCursor,
}

//struct IMEState
//{
//	ime_ofs: u8,
//	ime_val: u32,
//}

static MAIN_INPUT: InputChannel = InputChannel {
	shift_held: ModKeyPair::new(),
	ctrl_held: ModKeyPair::new(),
	alt_held: ModKeyPair::new(),
	//altgr: ModKeyPair::new(),
	cursor: MouseCursor::new(),
	};

pub fn init() {
	//MAIN_INPUT.cursor.
}

fn get_channel_by_index(_idx: usize) -> &'static InputChannel {
	&MAIN_INPUT
}

impl InputChannel
{
	pub fn handle_key(&self, key: keyboard::KeyCode, release: bool)
	{
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
		// - 'try_change_session' checks modifiers and permissions
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

		// Handle text events
		// - On key up, translate the keystroke into text (accounting for input state)
		// TODO: Support repetition?
		if release {
			//if self.enable_input_translation {
				let s = self.get_input_string(key);
				if s.len() > 0 {
					let mut buf = [0; 6];
					buf.clone_from_slice( s.as_bytes() );
					super::windows::handle_input( Event::Text(buf) );
				}
			//}
		}

		// TODO: Send key combination to active active window
		if release {
			super::windows::handle_input(/*self, */Event::KeyUp(key));
		}
		else {
			super::windows::handle_input(/*self, */Event::KeyDown(key));
		}
	}
	
	pub fn handle_mouse_move(&self, dx: i16, dy: i16)
	{
		// Mouse movement, update cursor
		self.cursor.move_pos(dx as i32, dy as i32);
		let (x,y) = self.cursor.pos();
		super::windows::handle_input(/*self, */Event::MouseMove(x, y, dx, dy));
	}
	pub fn handle_mouse_btn(&self, btn: u8, release: bool)
	{
		let (x,y) = self.cursor.pos();
		if release {
			super::windows::handle_input(/*self, */Event::MouseUp(x, y, btn));
		}
		else {
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
		ModKeyPair(ATOMIC_USIZE_INIT)
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
	fn move_pos(&self, dx: i32, dy: i32) {
		let mut lh = self.graphics_cursor.lock();
		let mut pos = lh.get_pos();
		pos.x = (pos.x as i32 + dx) as u32;
		pos.y = (pos.y as i32 + dy) as u32;
		lh.set_pos(pos);
	}
	fn pos(&self) -> (u32,u32) {
		let pos = self.graphics_cursor.lock().get_pos();
		(pos.x, pos.y)
	}
}
