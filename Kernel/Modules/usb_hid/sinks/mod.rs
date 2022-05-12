//! Input sinks

mod keyboard;
mod mouse;

pub use self::keyboard::Keyboard;
pub use self::mouse::Mouse;


/// A collection of sinks for a single device
#[derive(Default)]
pub struct Group
{
    pub keyboard: Option<Keyboard>,
    pub mouse: Option<Mouse>,
}

impl Group
{
    /// Populate the sink group by parsing a report specification
	pub fn from_report_spec(buf: &[u8]) -> Group
    {
        use crate::report_parser;
		let mut sinks = Group::default();

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

		fn input(&self, _sinks: &mut super::Group, _state: &ParseState, _bits: crate::report_parser::InputFlags) { }
		fn output(&self, _sinks: &mut super::Group, _state: &ParseState, _bits: u32) { }
		fn feature(&self, _sinks: &mut super::Group, _state: &ParseState, _bits: u32) { }
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
		fn input(&self, sinks: &mut super::Group, _state: &ParseState, bits: crate::report_parser::InputFlags) {
			log_debug!("Mouse Input {:?} ({:?})", bits, _state);
			if sinks.mouse.is_none() {
				sinks.mouse = Some(super::Mouse::new());
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
		fn input(&self, sinks: &mut super::Group, _state: &ParseState, bits: crate::report_parser::InputFlags) {
			log_debug!("Keyboard Input {:?} ({:?})", bits, _state);
			if sinks.keyboard.is_none() {
				sinks.keyboard = Some(super::Keyboard::new());
			}
		}
	}
}
