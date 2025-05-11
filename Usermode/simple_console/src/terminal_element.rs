// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// terminal_element.rs
//! Text terminal as a WTK element
use std::cell::RefCell;

pub struct TerminalElement<EventCb>
{
	inner: TerminalElementInner,
	cmd_callback: RefCell<EventCb>,
}
impl<EventCb> ::std::ops::Deref for TerminalElement<EventCb> {
	type Target = TerminalElementInner;
	fn deref(&self) -> &TerminalElementInner {
		&self.inner
	}
}

impl<EventCb> TerminalElement<EventCb>
where
	EventCb: FnMut(&mut dyn wtk::WindowTrait, &TerminalElementInner, ::syscalls::gui::Event)
{
	pub fn new(cb: EventCb) -> TerminalElement<EventCb> {
		TerminalElement {
			inner: TerminalElementInner::new(1024),
			cmd_callback: RefCell::new(cb),
			}
	}
}
impl<EventCb> ::wtk::Element for TerminalElement<EventCb>
where
	EventCb: FnMut(&mut dyn wtk::WindowTrait, &TerminalElementInner, ::syscalls::gui::Event)
{
	fn handle_event(&self, ev: ::wtk::InputEvent, win: &mut dyn wtk::WindowTrait) -> bool {
		let mut cb = self.cmd_callback.borrow_mut();
		(&mut *cb)(win, &self.inner, ev);
		true
	}
	fn resize(&self, _w: u32, _h: u32) {
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
		self.inner.surface.render(surface, force);
	}
	fn with_element_at_pos(&self, pos: ::wtk::geom::PxPos, _dims: ::wtk::geom::PxDims, f: ::wtk::WithEleAtPosCb) -> bool {
		f(self, pos)
	}
}

/// Element without the input-handling callback
pub struct TerminalElementInner
{
	surface: ::wtk_ele_console::TextConsole,
	insert_col: ::std::cell::Cell<Option<usize>>,
	cur_line: usize,
}
impl TerminalElementInner
{
	fn new(max_lines: usize) -> Self {
		TerminalElementInner {
			surface: ::wtk_ele_console::TextConsole::new(max_lines),
			insert_col: Default::default(),
			cur_line: 0,
		}
	}
}
impl super::Terminal for TerminalElementInner
{
	fn set_foreground(&self, col: ::wtk::Colour) {
		self.surface.append_fg_set(0, Some(col));
	}

	fn cur_col(&self) -> usize {
		self.insert_col.get().unwrap_or_else(|| {
			self.surface.line_len(self.cur_line)
		})
	}

	fn delete_left(&self) {
		if let Some(_) = self.insert_col.get() {
			panic!("TODO: delete_left");
		}
		else {
			self.surface.pop_from_line(self.cur_line);
		}
	}
	fn delete_right(&self) {
		if let Some(_) = self.insert_col.get() {
			panic!("TODO: delete_right");
		}
		else {
		}
	}

	fn cursor_left(&self) {
		self.insert_col.set(match self.insert_col.get() {
		Some(v) => {
			if v > 0 {
				Some(v - 1)
			}
			else {
				Some(v)
			}
		},
		None => {
			let l = self.surface.line_len(self.cur_line);
			if l > 0 {
				Some(l-1)
			}
			else {
				None
			}
		}
		})
	}
	fn cursor_right(&self) {
		self.insert_col.set(match self.insert_col.get() {
		Some(v) => {
			if v+1 == self.surface.line_len(self.cur_line) {
				None
			}
			else {
				Some(v+1)
			}
		},
		None => None,
		})
	}
	///// Shift line's data leftwards cursor onwards
	//fn shift_line_left(&mut self) {
	//	self.surf.shift_line_left(self.cur_col);
	//}
	///// Shift line's data rightwardss cursor onwards
	//fn shift_line_right(&mut self) {
	//	self.surf.shift_line_right(self.cur_col);
	//}
	
	fn write_str(&self, s: &str) {
		// Newlines need special handling, as the underlying element doesn't handle them
		for line in s.lines() {
			
			if let Some(v) = self.insert_col.get() {
				self.surface.insert_text(self.cur_line, v, format_args!("{}", line))
			}
			else {
				self.surface.append_text(self.cur_line, line);
			}

			self.surface.new_line();
			self.insert_col.set(None);
		}
	}
	fn write_fmt(&self, args: ::std::fmt::Arguments) {
		struct Out<'a> {
			parent: &'a TerminalElementInner,
			ss: StackString,
		}
		impl<'a> ::std::fmt::Write for Out<'a> {
			fn write_str(&mut self, s: &str) -> std::fmt::Result {
				for c in s.chars() {
					if !self.ss.push(c) {
						super::Terminal::write_str(self.parent, &self.ss);
						self.ss.clear();
					}
				}
				Ok( () )
			}
		}
		let mut o = Out { parent: self, ss: StackString::new() };
		let _ = ::std::fmt::write(&mut o, args);
		self.write_str(&o.ss);
	}
}


struct StackString {
	buffer: [u8; 128],
	len: usize,
}
impl StackString {
	pub const fn new() -> Self {
		StackString { buffer: [0; 128], len: 0 }
	}
	pub fn clear(&mut self) {
		self.len = 0;
	}
	pub fn push(&mut self, ch: char) -> bool {
		let mut tmp = [0; 4];
		let ch = ch.encode_utf8(&mut tmp);
		if self.len + ch.len() > self.buffer.len() {
			false
		}
		else {
			self.buffer[self.len..][..ch.len()].copy_from_slice(ch.as_bytes());
			true
		}
	}
}
impl ::std::ops::Deref for StackString {
	type Target = str;
	fn deref(&self) -> &Self::Target {
		// SAFE: This is created with valid UTF-8
		unsafe { ::std::str::from_utf8_unchecked(&self.buffer[..self.len]) }
	}
}
