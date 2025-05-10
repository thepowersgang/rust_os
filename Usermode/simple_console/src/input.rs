
use ::wtk::surface::UnicodeCombining;
use syscalls::gui::KeyCode;

pub enum Action<'a>
{
	Puts(&'a str),
	Backspace,
	Delete,
	CursorLeft,
	CursorRight,
}

#[derive(Default)]
pub struct InputStack
{
	/// Data from before the cursor
	buffer: String,
	/// Data from after the cursor
	tail_buffer: ::std::collections::VecDeque<u8>,
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
			KeyCode::Return | KeyCode::KpEnter => {
				self.buffer.push_str( std::str::from_utf8(self.tail_buffer.make_contiguous()).unwrap() );
				self.tail_buffer.clear();
				Some( ::std::mem::replace(&mut self.buffer, String::new()) )
			},
			KeyCode::Tab => {
				// TODO: Request a command completion
				//puts(Action::Complete(&self.buffer);
				None
				},
			KeyCode::Delete => {
				puts(Action::Delete);
				None
				},
			KeyCode::LeftArrow => {
				// Pop a cell (combining character set) from the and push into the after-
				// - Pops characters until a non-combining is popped
				loop {
					match self.buffer.pop()
					{
					None => break,
					Some(ch) => {
						self.push_after_char(ch);
						if !ch.is_combining() {
							break
						}
					}
					}
				}
				puts(Action::CursorLeft);
				None
			}
			KeyCode::RightArrow => {
				loop {
					// Pop the (should be non-combining) leader character
					match self.pop_after_char() {
					None => break,
					Some(ch) => {
						self.buffer.push(ch);
						// Then pop any combining characters afrer that
						// - When a non-combining character is seen, push it back into the buffer and stop
						while let Some(v) = self.pop_after_char() {
							if !v.is_combining() {
								self.push_after_char(v);
								break;
							}
							else {
								self.buffer.push(ch);
							}
						}
					}
					}
				}
				puts(Action::CursorRight);
				None
			}
			KeyCode::Backsp => {
				kernel_log!("Backspace");
				puts(Action::Backspace);
				// Pop while we're popping a combining character
				loop {
					match self.buffer.pop() {
					Some(v) if v.is_combining() => continue,
					_ => break,
					}
				}
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


	fn pop_after_char(&mut self) -> Option<char> {
		let mut buf = [0; 4];
		let mut i = 0;
		loop {
			match self.tail_buffer.pop_front() {
			None => break,
			Some(b) => {
				buf[i] = b;
				i += 1;
				if let Ok(v) = str::from_utf8(&buf) {
					return Some(v.chars().next().unwrap());
				}
			}
			}
		}
		None
	}
	fn push_after_char(&mut self, ch: char) {
		for b in ch.encode_utf8(&mut [0; 4]).bytes() {
			self.tail_buffer.push_front(b);
		}
	}
}

