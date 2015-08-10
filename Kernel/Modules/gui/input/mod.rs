// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/input/mod.rs
//! GUI input managment
use kernel::prelude::*;
use self::keyboard::KeyCode;
use core::atomic::{AtomicUsize,ATOMIC_USIZE_INIT,Ordering};

pub mod keyboard;

#[derive(Debug)]
pub enum Event
{
	KeyDown(keyboard::KeyCode),
	KeyUp(keyboard::KeyCode),
	MouseMove(i32,i32),
	MouseDown(u8),
	MouseUp(u8),
}

struct ModKeyPair(AtomicUsize);
const MOD_KEY_PAIR_INIT: ModKeyPair = ModKeyPair(ATOMIC_USIZE_INIT);

struct MouseCursor(u32,u32);
const MOUSE_CURSOR_INIT: MouseCursor = MouseCursor(0,0);

struct InputChannel
{
	shift_held: ModKeyPair,
	ctrl_held: ModKeyPair,
	alt_held: ModKeyPair,
	
	cursor: MouseCursor,
}

static MAIN_INPUT: InputChannel = InputChannel {
	shift_held: MOD_KEY_PAIR_INIT,
	ctrl_held: MOD_KEY_PAIR_INIT,
	alt_held: MOD_KEY_PAIR_INIT,
	cursor: MOUSE_CURSOR_INIT,
	};

fn get_channel_by_index(_idx: usize) -> &'static InputChannel {
	&MAIN_INPUT
}

impl InputChannel
{
	pub fn handle(&self, event: Event)
	{
		log_debug!("handle({:?})", event);
		match event
		{
		// Maintain key states
		Event::KeyDown(KeyCode::RightShift) => self.shift_held.set_r(),
		Event::KeyDown(KeyCode::LeftShift)  => self.shift_held.set_l(),
		Event::KeyDown(KeyCode::RightCtrl) => self.ctrl_held.set_r(),
		Event::KeyDown(KeyCode::LeftCtrl)  => self.ctrl_held.set_l(),
		Event::KeyDown(KeyCode::RightAlt) => self.alt_held.set_r(),
		Event::KeyDown(KeyCode::LeftAlt)  => self.alt_held.set_l(),
		Event::KeyUp(KeyCode::RightShift) => self.shift_held.clear_r(),
		Event::KeyUp(KeyCode::LeftShift)  => self.shift_held.clear_l(),
		Event::KeyUp(KeyCode::RightCtrl) => self.ctrl_held.clear_r(),
		Event::KeyUp(KeyCode::LeftCtrl)  => self.ctrl_held.clear_l(),
		Event::KeyUp(KeyCode::RightAlt) => self.alt_held.clear_r(),
		Event::KeyUp(KeyCode::LeftAlt)  => self.alt_held.clear_l(),
		// Check for session change commands, don't propagate if they fired
		Event::KeyDown(KeyCode::Esc) => if self.try_change_session(0) { return ; },
		Event::KeyDown(KeyCode::F1)  => if self.try_change_session(1) { return ; },
		Event::KeyDown(KeyCode::F2)  => if self.try_change_session(2) { return ; },
		Event::KeyDown(KeyCode::F3)  => if self.try_change_session(3) { return ; },
		Event::KeyDown(KeyCode::F4)  => if self.try_change_session(4) { return ; },
		Event::KeyDown(KeyCode::F5)  => if self.try_change_session(5) { return ; },
		Event::KeyDown(KeyCode::F6)  => if self.try_change_session(6) { return ; },
		Event::KeyDown(KeyCode::F7)  => if self.try_change_session(7) { return ; },
		Event::KeyDown(KeyCode::F8)  => if self.try_change_session(8) { return ; },
		Event::KeyDown(KeyCode::F9)  => if self.try_change_session(9) { return ; },
		Event::KeyDown(KeyCode::F10) => if self.try_change_session(10) { return ; },
		Event::KeyDown(KeyCode::F11) => if self.try_change_session(11) { return ; },
		Event::KeyDown(KeyCode::F12) => if self.try_change_session(12) { return ; },
		// Mouse movement, update cursor
		Event::MouseMove(dx,dy) => self.cursor.move_pos(dx, dy),
		
		_ => {},
		}
		
		// TODO: Send key combination to active active window
		super::windows::handle_input(/*self, */event);
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
	fn set_l(&self) { self.0.fetch_or(1, Ordering::Relaxed); }
	fn set_r(&self) { self.0.fetch_or(2, Ordering::Relaxed); }
	fn clear_l(&self) { self.0.fetch_and(!1, Ordering::Relaxed); }
	fn clear_r(&self) { self.0.fetch_and(!2, Ordering::Relaxed); }
	fn get(&self) -> bool {
		self.0.load(Ordering::Relaxed) != 0
	}
}
impl MouseCursor {
	fn move_pos(&self, dx: i32, dy: i32) {
		// TODO
		todo!("Mouse move by {},{}", dx, dy);
	}
}
