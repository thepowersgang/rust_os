

use syscalls::gui::KeyCode;

pub enum Action<'a>
{
	Puts(&'a str),
	Backspace,
	Delete,
}

#[derive(Default)]
pub struct InputStack
{
	//	caps: bool,	// Caps can DIAF
	shift: u8,
	ctrl: u8,
	alt: u8,
	altgr: bool,

	ime_ofs: u8,
	ime_val: u32,

	buffer: String,
}


impl InputStack
{
	pub fn new() -> InputStack {
		InputStack::default()
	}
	pub fn handle_key<F: FnOnce(Action)>(&mut self, release: bool, keycode: u8, puts: F) -> Option<String>
	{
		kernel_log!("handle_key: (release={},keycode={},...)", release, keycode);
		if release {
			match KeyCode::from(keycode)
			{
			KeyCode::Return | KeyCode::KpEnter => Some( ::std::mem::replace(&mut self.buffer, String::new()) ),
			KeyCode::LeftShift => { self.shift &= !1; None },
			KeyCode::RightShift => { self.shift &= !2; None },
			KeyCode::Backsp => {
				kernel_log!("Backspace");
				puts(Action::Backspace);
				self.buffer.pop();	// TODO: Pop a grapheme, not just a char
				kernel_log!("- self.buffer = {:?}", self.buffer);
				None
				},
			kc @ _ => {
				let val = self.get_string(kc);
				kernel_log!("val={:?}", val);
				if val != "" {
					puts(Action::Puts(val));
					self.buffer.push_str(val);
				}
				None
				},
			}
		}
		else {
			match KeyCode::from(keycode)
			{
			KeyCode::LeftShift => { self.shift |= 1; },
			KeyCode::RightShift => { self.shift |= 2; },
			_ => {},
			}
			None
		}
	}

	fn get_string(&self, keycode: KeyCode) -> &'static str
	{
		macro_rules! alpha { ($s:ident: $lower:expr, $upper:expr) => { if $s.upper() { $upper } else {$lower} }; }
		match keycode
		{
		KeyCode::A => alpha! { self: "a", "A" },
		KeyCode::B => alpha! { self: "b", "B" },
		KeyCode::C => alpha! { self: "c", "C" },
		KeyCode::D => alpha! { self: "d", "D" },
		KeyCode::E => alpha! { self: "e", "E" },
		KeyCode::F => alpha! { self: "f", "F" },
		KeyCode::G => alpha! { self: "g", "G" },
		KeyCode::H => alpha! { self: "h", "H" },
		KeyCode::I => alpha! { self: "i", "I" },
		KeyCode::J => alpha! { self: "j", "J" },
		KeyCode::K => alpha! { self: "k", "K" },
		KeyCode::L => alpha! { self: "l", "L" },
		KeyCode::M => alpha! { self: "m", "M" },
		KeyCode::N => alpha! { self: "n", "N" },
		KeyCode::O => alpha! { self: "o", "O" },
		KeyCode::P => alpha! { self: "p", "P" },
		KeyCode::Q => alpha! { self: "q", "Q" },
		KeyCode::R => alpha! { self: "r", "R" },
		KeyCode::S => alpha! { self: "s", "S" },
		KeyCode::T => alpha! { self: "t", "T" },
		KeyCode::U => alpha! { self: "u", "U" },
		KeyCode::V => alpha! { self: "v", "V" },
		KeyCode::W => alpha! { self: "w", "W" },
		KeyCode::X => alpha! { self: "x", "X" },
		KeyCode::Y => alpha! { self: "y", "Y" },
		KeyCode::Z => alpha! { self: "z", "Z" },
		KeyCode::Space => " ",
		_ => "",
		}

	}

	fn upper(&self) -> bool { self.shift() }
	fn shift(&self) -> bool { self.shift != 0 }
}

