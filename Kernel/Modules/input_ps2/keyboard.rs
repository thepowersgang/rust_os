// "Tifflin" Kernel - ATA Driver
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/keyboard.rs
//! PS2 Keyboard driver
use kernel::_common::*;

#[derive(Debug)]
pub enum Type
{
	AT,
	MF2,
	MF2Emul,
}

#[derive(Debug)]
enum State
{
	Init(Init),
	Idle(Layer,bool),
}
#[derive(Copy,Clone,Debug)]
enum Layer
{
	Base,
	E0,
	E1,
}
#[derive(Copy,Clone,Debug)]
enum Init
{
	Disabled,
	ReqScancodeSetAck,
	ReqScancodeSetRsp,
	SetLeds(u8),
}

#[derive(Debug)]
pub struct Dev
{
	ty: Type,
	state: State,
}

impl Dev
{
	pub fn new(ty: Type) -> (Option<u8>,Dev) {
		
		match ty
		{
		Type::AT => {
			log_warning!("Unexpected AT keyboard");
			return (None, Dev { ty: ty, state: State::Init(Init::Disabled) });
			},
		Type::MF2Emul => {
			log_warning!("Unexpected emulation enabled MF2");
			return (None, Dev { ty: ty, state: State::Init(Init::Disabled) });
			},
		Type::MF2 => {},
		}
		
		// 1. Request scancode set
		(Some(0xF0), Dev {
			ty: ty,
			state: State::Init(Init::ReqScancodeSetAck),
			})
	}
	
	pub fn recv_byte(&mut self, byte: u8) -> Option<u8> {
		match self.state
		{
		State::Init(s) =>
			match s
			{
			Init::Disabled => None,
			Init::ReqScancodeSetAck => {
				self.state = State::Init(Init::ReqScancodeSetRsp);
				Some(0x00)
				},
			Init::ReqScancodeSetRsp =>
				match byte
				{
				// Scancode set 1
				1 /*0x43*/ => {
					log_warning!("TODO: Support scancode set 1");
					self.state = State::Init(Init::Disabled);
					None
					},
				// Scancode set 2 (most common)
				2 /*0x41*/ => {
					self.state = State::Idle(Layer::Base,false);
					None
					},
				// Scancode set 3 (newest)
				3 /*0x3F*/ => {
					log_warning!("TODO: Support scancode set 3");
					self.state = State::Init(Init::Disabled);
					None
					},
				_ => {
					log_warning!("Unkown scancode set reponse {:#02x}", byte);
					self.state = State::Init(Init::Disabled);
					None
					},
				},
			Init::SetLeds(v) => {
				self.state = State::Idle(Layer::Base,false);
				Some(v)
				},
			},
		State::Idle(layer,mut release) =>
			match byte
			{
			// Error/Buffer Overrun
			0x00 => None,
			0xFF => None,
			// Self-test passed
			0xAA => None,
			// Self-test failed
			0xFC => None,
			0xFD => None,
			// Echo reply
			0xEE => None,
			// ACK
			0xFA => { log_notice!("Unexpected ACK from keyboard"); None },
			// Resend
			0xFE => { log_notice!("Resend request from keyboard"); None },
			// Extended scancodes
			0xE0 => {
				self.state = State::Idle(Layer::E0, false);
				None
				},
			0xE1 => {
				self.state = State::Idle(Layer::E1, false);
				None
				},
			// Released key flag
			0xF0 => {
				self.state = State::Idle(layer, true);
				None
				},
			
			v @ _ => {
				log_debug!("Scancode {:?} {:#02x} (release={})", layer, v, release);
				// TODO: Translate to a HID scancode, then pass to a higher layer (GUI or metadev keyboard)
				self.state = State::Idle(Layer::Base,false);
				None
				},
			},
		}
	}
}

