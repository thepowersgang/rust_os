

use terminal_surface::Surface;
use syscalls::gui::{Window,Rect,Colour};

pub struct Terminal<'a>
{
	surf: Surface<'a>,
	cur_col: usize,
	cur_row: usize,
	cur_fg: Colour,
}

impl<'a> Terminal<'a>
{
	pub fn new(window: &Window, pos: Rect) -> Terminal {
		Terminal {
			surf: Surface::new(window, pos),
			cur_row: 0,
			cur_col: 0,
			cur_fg: Colour::white(),
		}
	}
	
	pub fn flush(&mut self) {
		self.surf.flush();
	}
}

impl<'a> ::std::fmt::Write for Terminal<'a>
{
	fn write_str(&mut self, s: &str) -> ::std::fmt::Result
	{
		for c in s.chars() {
			try!(self.write_char(c));
		}
		Ok( () )
	}

	fn write_char(&mut self, c: char) -> ::std::fmt::Result {
		if c == '\n' {
			self.cur_row += 1;
			assert!(self.cur_row < self.surf.max_rows());
			self.surf.set_row(self.cur_row);
		}
		else {
			self.cur_col += if self.surf.putc(self.cur_col, self.cur_fg, c) { 1 } else { 0 };
		}
		Ok( () )
	}
}


