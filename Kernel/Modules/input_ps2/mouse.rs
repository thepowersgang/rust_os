// "Tifflin" Kernel - ATA Driver
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/mouse.rs
//! PS2 Mouse driver
use gui::input::mouse as gui_mouse;

#[derive(Debug)]
pub enum Type
{
	Std,
	Scroll,
	QuintBtn,	// 5 buttons
}

#[derive(Debug)]
enum State
{
	Idle,
	// TODO: Initialise mouse to have a know config
	// TODO: Support magic to switch types up to scroll / five-button
	WaitByte2(u8),
	WaitByte3(u8,u8),
}

#[derive(Debug)]
pub struct Dev
{
	ty: Type,
	state: State,
	guidev: gui_mouse::Instance,
	btns: u8,
}

impl Dev
{
	pub fn new(ty: Type) -> (Option<u8>,Dev) {
		(None, Dev {
			ty: ty,
			state: State::Idle,
			guidev: gui_mouse::Instance::new(),
			btns: 0x00,
			})
	}
	
	pub fn recv_byte(&mut self, byte: u8) -> Option<u8> {
		let (rv, ns) = match self.state
			{
			State::Idle =>
				(None, State::WaitByte2(byte)),
			State::WaitByte2(b1) =>
				(None, State::WaitByte3(b1, byte)),
			State::WaitByte3(b1, b2) => {
				assert!(is!(self.ty, Type::Std));
				let newbtns = b1 & 0b111;
				let dx = Self::get_signed_9( ((b1 >> 6) & 1) != 0, ((b1 >> 4) & 1) != 0, b2 );
				let dy = Self::get_signed_9( ((b1 >> 7) & 1) != 0, ((b1 >> 5) & 1) != 0, byte );
				log_trace!("btns = {:#x}, (dx,dy) = ({},{})", newbtns, dx, dy);

				if dx != 0 || dy != 0 {
					self.guidev.move_cursor(dx, dy);
				}
				if newbtns != self.btns {
					for i in 0 .. 8 {
						match ( (self.btns & 1 << i) != 0, (newbtns & 1 << i) != 0 ) {
						(false, false) => {},
						(false, true ) => self.guidev.press_button(i as u8),
						(true , true ) => {},
						(true , false) => self.guidev.release_button(i as u8),
						}
					}
				}
				(None, State::Idle)
				},
			};

		self.state = ns;
		rv
	}


	fn get_signed_9(overflow: bool, sign: bool, val: u8) -> i16 {
		if sign {
			if overflow {
				-256
			}
			else {
				-(val as i16)
			}
		}
		else {
			if overflow {
				256
			}
			else {
				(val as i16)
			}
		}
	}
}


