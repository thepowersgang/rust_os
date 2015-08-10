

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
	buffer: String,
}


impl InputStack
{
	pub fn new() -> InputStack {
		InputStack::default()
	}
	pub fn handle_event<F: FnOnce(Action)>(&mut self, ev: ::syscalls::gui::Event, puts: F) -> Option<String>
	{
		kernel_log!("handle_key: (ev={:?},...)", ev);
		match ev
		{
		::syscalls::gui::Event::KeyUp(keycode) =>
			match KeyCode::from(keycode as u8)
			{
			KeyCode::Return | KeyCode::KpEnter => Some( ::std::mem::replace(&mut self.buffer, String::new()) ),
			KeyCode::Tab => {
				//puts(Action::Complete(&self.buffer);
				None
				},
			KeyCode::Delete => {
				puts(Action::Delete);
				None
				},
			KeyCode::Backsp => {
				kernel_log!("Backspace");
				puts(Action::Backspace);
				self.buffer.pop();	// TODO: Pop a grapheme, not just a char
				kernel_log!("- self.buffer = {:?}", self.buffer);
				None
				},
			_ => {
				None
				},
			},
		::syscalls::gui::Event::KeyDown(keycode) =>
			match KeyCode::from(keycode as u8)
			{
			_ => None,
			},
		::syscalls::gui::Event::Text(val) => {
			self.buffer.push_str(&val);
			puts( Action::Puts(&val) );
			None
			},
		_ => None,
		}
	}
}

