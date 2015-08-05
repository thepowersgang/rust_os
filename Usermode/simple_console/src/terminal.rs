

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

	pub fn set_foreground(&mut self, col: Colour) {
		self.cur_fg = col;
	}
	
	pub fn flush(&mut self) {
		self.surf.draw_cursor(self.cur_col);
		self.surf.flush();
		self.surf.clear_cursor(self.cur_col);
	}

	pub fn cur_col(&self) -> usize { self.cur_col }

	pub fn delete_left(&mut self) {
		if self.cur_col > 0 {
			self.cursor_left();
			self.shift_line_left();
		}
	}
	pub fn delete_right(&mut self) {
		self.shift_line_left();
	}

	pub fn cursor_left(&mut self) {
		assert!(self.cur_col != 0);
		self.cur_col -= 1;
	}
	/// Shift line's data leftwards cursor onwards
	pub fn shift_line_left(&mut self) {
		self.surf.shift_line_left(self.cur_col);
	}
	/// Shift line's data rightwardss cursor onwards
	pub fn shift_line_right(&mut self) {
		self.surf.shift_line_right(self.cur_col);
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
			self.cur_col = 0;
			assert!(self.cur_row < self.surf.max_rows());
			self.surf.set_row(self.cur_row);
		}
		else {
			self.cur_col += if self.surf.putc(self.cur_col, self.cur_fg, c) { 1 } else { 0 };
		}
		Ok( () )
	}
}


