// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/lib.rs
//! PS2 Keyboard/Mouse controller
#![feature(linkage)]
#![no_std]

#[macro_use]
extern crate kernel;

extern crate gui;

#[allow(unused_imports)]
use kernel::prelude::*;

// HACK: Requires USB to be active to ensure that emulation is off
module_define!{PS2, [
	DeviceManager,
	#[cfg(any(arch="x86",arch="amd64",target_arch="x86",target_arch="x86_64"))]
	ACPI,
	GUI
	/*, USB*/
	], init}

#[derive(Debug)]
enum PS2Dev
{
	None,
	Unknown,
	Enumerating(EnumWaitState),
	Keyboard(keyboard::Dev),
	Mouse(mouse::Dev),
}
impl Default for PS2Dev { fn default() -> Self { PS2Dev::None } }

#[derive(Copy,Clone,Debug)]
enum EnumWaitState
{
	DSAck,
	IdentAck,
	IdentB1,
	IdentB2(u8),
}

mod keyboard;
mod mouse;

#[cfg(any(arch="x86",arch="amd64",target_arch="x86",target_arch="x86_64"))]
fn init()
{
	#[path="i8042.rs"]
	mod i8042;
	i8042::init();
}
#[cfg(any(target_arch="arm",target_arch="aarch64"))]
fn init()
{
	#[path="pl050.rs"]
	mod pl050;
	pl050::init();
}
#[cfg(any(target_arch="riscv64"))]
fn init()
{
	// Is there a PS/2 controller
	
	// Minimal usage of the driver interfaces to prevent warnings
	#[allow(dead_code)]
	fn unused() {
		let mut dev = PS2Dev::None;
		dev.recv_byte(0);
	}
}

impl PS2Dev
{
	fn new_mouse(ty: mouse::Type) -> (Option<u8>, Option<PS2Dev>) {
		let (byte, dev) = mouse::Dev::new(ty);
		(byte, Some(PS2Dev::Mouse(dev)))
	}
	fn new_keyboard(ty: keyboard::Type) -> (Option<u8>, Option<PS2Dev>) {
		let (byte, dev) = keyboard::Dev::new(ty);
		(byte, Some(PS2Dev::Keyboard(dev)))
	}
	
	/// Handle a recieved byte, and optionally return a byte to be sent to the device
	pub fn recv_byte(&mut self, byte: u8) -> Option<u8> {
		let (rv, new_state): (Option<_>,Option<_>) = match *self
			{
			PS2Dev::None =>
				// TODO: Clean this section up, the OSDev.org wiki is a little hazy on the ordering
				if byte == 0xFA {
					(None, None)
				}
				else if byte == 0xAA {
					// Send 0xF5 "Disable Scanning" and wait for ACK
					(Some(0xF5), Some(PS2Dev::Enumerating(EnumWaitState::DSAck)))
				}
				else {
					(None, None)
				},
			PS2Dev::Unknown => (None, None),
			PS2Dev::Enumerating(state) => match state
				{
				EnumWaitState::DSAck =>
					if byte == 0xFA {
						// Send 0xF2 "Identify"
						(Some(0xF2), Some(PS2Dev::Enumerating(EnumWaitState::IdentAck)))
					}
					else if byte == 0x00 {
						// XXX: Ignore spurrious NUL byte
						(None, None)
					}
					else {
						(None, Some(PS2Dev::Unknown))
					},
				EnumWaitState::IdentAck =>
					if byte == 0xFA {
						// TODO: Start a timeout if not enough bytes are sent
						(None, Some(PS2Dev::Enumerating(EnumWaitState::IdentB1)))
					}
					else {
						(None, Some(PS2Dev::Unknown))
					},
				EnumWaitState::IdentB1 =>
					match byte
					{
					0x00 => Self::new_mouse(mouse::Type::Std),
					0x03 => Self::new_mouse(mouse::Type::Scroll),
					0x04 => Self::new_mouse(mouse::Type::QuintBtn),
					0xAB => (None, Some(PS2Dev::Enumerating(EnumWaitState::IdentB2(byte)))),
					_ => {
						log_warning!("Unknown PS/2 device {:#02x}", byte);
						(None, Some(PS2Dev::Unknown))
						},
					},
				EnumWaitState::IdentB2(b1) =>
					match (b1,byte)
					{
					(0xAB, 0x83) => Self::new_keyboard(keyboard::Type::MF2),
					(0xAB, 0x41) => Self::new_keyboard(keyboard::Type::MF2Emul),
					(0xAB, 0xC1) => Self::new_keyboard(keyboard::Type::MF2Emul),
					_ => {
						log_warning!("Unknown PS/2 device {:#02x} {:#02x}", b1, byte);
						(None, Some(PS2Dev::Unknown))
						},
					},
				},
			PS2Dev::Keyboard(ref mut dev) => {
				(dev.recv_byte(byte), None)
				},
			PS2Dev::Mouse(ref mut dev) => {
				(dev.recv_byte(byte), None)
				},
			};
		
		if let Some(ns) = new_state
		{
			log_debug!("Byte {:#02x} caused State transition {:?} to {:?}", byte, *self, ns);
			*self = ns;
		}
		rv
	}
}

