// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/keyboard.rs
//! PS2 Keyboard driver
use gui::input::keyboard as gui_keyboard;
use gui::input::keyboard::KeyCode;

#[derive(Debug)]
pub enum Type
{
	//AT,
	MF2,
	MF2Emul,
}

#[derive(Debug)]
enum State
{
	Disabled,
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
	/// 0xF0 sent, waiting for 0xFA
	ReqScancodeSetAck,
	/// 0x00 sent, waiting for code (may also get 0xFA)
	ReqScancodeSetRsp,
	/// 0xF4 sent, waiting for 0xFA
	EnableAck,
}

#[derive(Debug)]
pub struct Dev
{
	_ty: Type,
	state: State,
	guidev: gui_keyboard::Instance,
}

impl Dev
{
	/// Create a new keyboard driver instance
	///
	/// Returns a byte to send to the device
	pub fn new(ty: Type) -> (Option<u8>,Dev) {
		match ty
		{
		//Type::AT => {
		//	log_warning!("Unexpected AT keyboard");
		//	return (None, Dev { ty: ty, state: State::Disabled, guidev: Default::default() });
		//	},
		Type::MF2Emul => {
			log_warning!("Unexpected emulation enabled MF2");
			return (None, Dev { _ty: ty, state: State::Disabled, guidev: Default::default() });
			},
		Type::MF2 => {
			// 1. Request scancode set
			(Some(0xF0), Dev {
				_ty: ty,
				state: State::Init(Init::ReqScancodeSetAck),
				guidev: gui_keyboard::Instance::new(),
				})
			},
		}
	}
	
	/// Handle a received byte
	///	
	/// Returns a response byte
	pub fn recv_byte(&mut self, byte: u8) -> Option<u8> {
		match self.state
		{
		State::Disabled => {
			log_debug!("Disabled keyboard {:#02x}", byte);
			None
			},
		// Non-active states (mostly initiailsation)
		State::Init(s) =>
			match s
			{
			Init::ReqScancodeSetAck => {
				log_debug!("ACK ReqScancodeSet");
				self.state = State::Init(Init::ReqScancodeSetRsp);
				Some(0x00)
				},
			Init::ReqScancodeSetRsp =>
				match byte
				{
				// Scancode set 1
				1 /*0x43*/ => {
					log_warning!("TODO: Support scancode set 1");
					self.state = State::Disabled;
					None
					},
				// Scancode set 2 (most common)
				2 /*0x41*/ => {
					log_debug!("Keyboard ready, scancode set 2");
					self.state = State::Init(Init::EnableAck);
					Some(0xF4)	// Enable scanning
					},
				// Scancode set 3 (newest)
				3 /*0x3F*/ => {
					log_warning!("TODO: Support scancode set 3");
					self.state = State::Disabled;
					None
					},
				0xFA => {
					// This is kinda expected?
					log_warning!("Received second ACK for ReqScancodeSetRsp {:#02x}", byte);
					None
					},
				_ => {
					log_warning!("Unkown scancode set reponse {:#02x}", byte);
					self.state = State::Disabled;
					None
					},
				},
			Init::EnableAck =>
				match byte
				{
				0xFA => {
					self.state = State::Idle(Layer::Base,false);
					None
					},
				_ => {
					log_warning!("Unkown scancode set reponse {:#02x}", byte);
					self.state = State::Disabled;
					None
					},
				},
			},
		// Idle and ready to process keystrokes
		State::Idle(layer,release) =>
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
				// Translate to a HID scancode
				let mapping: &[KeyCode] = match layer
					{
					Layer::Base => &keymaps::SC2_BASE,
					Layer::E0 => &keymaps::SC2_E0,
					_ => &[],
					};
				let key = *mapping.get(v as usize).unwrap_or(&KeyCode::None);
				if key == KeyCode::None {
					if ! release {
						log_warning!("Scancode {:?} {:#02x} has no mapping", layer, v);
					}
				}
				else {
					if release {
						self.guidev.release_key(key);
					}
					else {
						self.guidev.press_key(key);
					}
				}
				self.state = State::Idle(Layer::Base,false);
				None
				},
			},
		}
	}
}


mod keymaps {
	use gui::input::keyboard::KeyCode;
	use gui::input::keyboard::KeyCode::*;
	pub static SC2_BASE: [KeyCode; 0x88] = [
		None, F9,  None, F5, F3, F1 , F2, F12,
		None, F10, F8,   F6, F4, Tab, GraveTilde, None,
		None, LeftAlt, LeftShift, None, LeftCtrl, Q, Kb1, None,
		None, None , Z, S, A, W  , Kb2, None,
		None, C    , X, D, E, Kb4, Kb3, None,
		None, Space, V, F, T, R  , Kb5, None,
		None, N    , B, H, G, Y  , Kb6, None,
		None, None , M, J, U, Kb7, Kb8, None,
		None, Comma, K, I, O, Kb0, Kb9, None,
		None, Period, Slash, L, Semicolon, P, Minus, None,
		None, None, Quote, None, SquareOpen, Equals, None, None,
		Caps, RightShift, Return, SquareClose, None, Backslash, None, None,
		None, None, None, None,  None, None, Backsp, None,
		None, Kp1 , None, Kp4 ,  Kp7 , None, None, None,
		Kp0 , KpPeriod, Kp2, Kp5    , Kp6   , Kp8, Esc       , Numlock,
		F11 , KpPlus  , Kp3, KpMinus, KpStar, Kp9, ScrollLock, None,
		None, None, None, F7, None, None, None, None,
		];
	// TODO: There's a chunk of multimedia/WWW keys in here that I don't know the HID codes for
	pub static SC2_E0: [KeyCode; 0x80] = [
		None, None, None, None, None, None, None, None,
		None, None, None, None, None, None, None, None,
		None, RightAlt, None, None, RightCtrl, None, None, None,
		None, None, None, None, None, None, None, LeftGui,
		None, None, None, None, None, None, None, RightGui,
		None, None, None, None, None, None, None, Application,	// Application = Menu
		None, None, None, None, None, None, None, None,
		None, None, None, None, None, None, None, None,
		None, None, None, None, None, None, None, None,
		None, None, KpSlash, None, None, None, None, None,
		None, None, None, None, None, None, None, None,
		None, None, KpEnter, None, None, None, None, None,
		None, None, None, None, None, None, None, None,
		None, End , None, LeftArrow, Home, None, None, None,
		Insert, Delete, DownArrow, None, RightArrow, UpArrow, None, None,
		None, None, PgDn, None, None, PgUp, None, None,
		];
	// TODO: E1 (only contains pause/break)
}

