// Tifflin OS - WTK Element: Text Console
// - By John Hodge (thePowersGang)
//
// src/lib.rs
//! Text terminal as a WTK element
// TODO: This could maybe be split up into a rendering component and a backend with the line data?
use ::wtk::Colour;
use ::wtk::geom::Rect;
use std::cell::RefCell;

const COLOUR_DEFAULT_BG: u32 = 0x33_00_00;	// An ochre red
const COLOUR_DEFAULT_FG: u32 = 0xFF_FF_FF;	// White
const FONT_HEIGHT: u32 = 16;
const FONT_WIDTH : u32 = 8;

mod encoded_line;
use self::encoded_line::{Line,LineEnt};

/// The text console element
pub struct TextConsole
{
	lines: RefCell<Lines>,
	max_lines: usize,
	render_cache: RefCell<RenderCache>,
}
#[derive(Default)]
struct Lines {
	/// The line data
	lines: Vec<Line>,
	/// Cursor location (line)
	cursor_line: usize,
	/// Cursor location (cell)
	cursor_cell: usize,
}
#[derive(Default)]
struct RenderCache {
	line_count: usize,
}

impl TextConsole
{
	pub fn new(max_lines: usize) -> Self
	{
		let rv = TextConsole {
			lines: Default::default(),
			max_lines,
			render_cache: Default::default(),
			};
		rv.lines.borrow_mut().cursor_line = !0;
		rv.new_line();
		rv
	}

	/// Get the length of this line in terms of render cells
	pub fn line_len(&self, line: usize) -> usize {
		let mut len = 0;
		self.with_line(line, |l|{
			len = l.num_cells();
		});
		len
	}
	pub fn cursor_line(&self) -> usize {
		self.lines.borrow().cursor_line
	}
	pub fn cursor_cell(&self) -> usize {
		self.lines.borrow().cursor_cell
	}

	
	pub fn pop_from_line(&self, line: usize) {
		self.with_line(line, |l| {
			l.delete_cell_back();
		});
	}

	/// Push a new line onto the end of the console, potentially scrolling the display
	pub fn new_line(&self) {
		let mut lh = self.lines.borrow_mut();
		lh.lines.push(Line::default());
		if lh.lines.len() >= self.max_lines {
			lh.lines.remove(0);
		}
	}
	/// Insert a new blank line at a specified offset from the bottom
	pub fn insert_line(&self, location: usize) {
		let mut lh = self.lines.borrow_mut();
		let index = lh.lines.len() - 1 - location;
		lh.lines.insert(index, Line::default());
	}
	/// Remove a line entirely
	pub fn remove_line(&self, line: usize) {
		let mut lh = self.lines.borrow_mut();
		let line = lh.lines.len() - 1 - line;
		lh.lines.remove(line);
	}

	fn with_line(&self, line: usize, fcn: impl FnOnce(&mut Line)) {
		let mut lh = self.lines.borrow_mut();
		let line = lh.lines.len() - 1 - line;
		let line = &mut lh.lines[line];
		fcn(line)
	}
	/// Erase the contents of a line
	pub fn erase_line(&self, line: usize) {
		self.with_line(line, |line| *line = Line::default());
	}
	/// Set the background colour for all characters after the current position
	pub fn append_bg_set(&self, line: usize, colour: Option<Colour>) {
		self.with_line(line, |line| {
			line.append_bg(colour.unwrap_or(Colour::from_argb32(COLOUR_DEFAULT_BG)));
		});
	}
	/// Set the foreground colour for all characters after the current position
	pub fn append_fg_set(&self, line: usize, colour: Option<Colour>) {
		self.with_line(line, |line| {
			line.append_fg(colour.unwrap_or(Colour::from_argb32(COLOUR_DEFAULT_FG)));
		});
	}
	/// Append text onto the end of a line (using a `&str`)
	pub fn append_text(&self, line: usize, text: &str) {
		self.with_line(line, |line| {
			line.append_text(text);
		});
	}
	/// Append text onto the end of a line (using an iterator of characters)
	pub fn append_chars(&self, line: usize, text: impl Iterator<Item=char>) {
		self.with_line(line, |line| {
			line.append_iter(text);
		});
	}
	/// Append text onto the end of a line (using `format_args`)
	pub fn append_fmt(&self, line: usize, args: ::std::fmt::Arguments) {
		self.with_line(line, |line| {
			let _ = ::std::fmt::Write::write_fmt(&mut AppendFmt(line), args);
		});
	}

	/// Insert text before a specific cell (with the same formatting as that cell)
	pub fn insert_text(&self, line: usize, cell: usize, args: ::std::fmt::Arguments) {
		self.with_line(line, |line| {
			let tail = line.split_off_at(cell);
			let _ = ::std::fmt::Write::write_fmt(&mut AppendFmt(line), args);
			line.append(tail);
		});
	}
}
struct AppendFmt<'a>(&'a mut Line);
impl<'a> ::std::fmt::Write for AppendFmt<'a> {
	fn write_str(&mut self, s: &str) -> ::std::fmt::Result {
		self.0.append_text(s);
		Ok(())
	}
}
impl ::wtk::Element for TextConsole
{
	fn handle_event(&self, _ev: ::wtk::InputEvent, _win: &mut dyn wtk::WindowTrait) -> bool {
		false
	}
	fn resize(&self, _w: u32, _h: u32) {
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, mut force: bool) {

		let mut rc_h = self.render_cache.borrow_mut();
		let backing = self.lines.borrow();

		force |= rc_h.line_count != backing.lines.len();
		::syscalls::kernel_log!("TextConsole::render: force={}", force);
		if force {
			surface.fill_rect(Rect::new_full(), Colour::from_argb32(COLOUR_DEFAULT_BG));
		}

		rc_h.line_count = backing.lines.len();
	
		let mut dest_line = surface.height() / FONT_HEIGHT;
		let line_width = surface.width() / FONT_WIDTH;
		for (idx, line) in backing.lines.iter().enumerate().rev()
		{
			if line.take_is_dirty() || force
			{
				let num_lines = (line.num_cells() as u32 + line_width - 1) / line_width;
				dest_line -= num_lines;
				let mut x = 0;
				let y = dest_line * FONT_HEIGHT;
				let cursor_x = if idx == backing.cursor_line {
					Some(backing.cursor_cell as u32 * FONT_WIDTH)
				} else {
					None
				};

				let mut fg = Colour::from_argb32(COLOUR_DEFAULT_FG);
				let mut bg = Colour::from_argb32(COLOUR_DEFAULT_BG);
				for seg in line.segs(0)
				{
					match seg
					{
					LineEnt::Text(text) => {
						let nc = encoded_line::rendered_cell_count(text);
						surface.fill_rect( Rect::new(x,y, nc as u32 * FONT_WIDTH, FONT_HEIGHT), bg );
						let w = surface.draw_text( Rect::new(x,y, !0,!0), text.chars(), fg );
						if let Some(cursor_x) = cursor_x {
							if x <= cursor_x && cursor_x < x + w as u32 {
								// Draw a vertical bar as cursor
								surface.fill_rect( Rect::new(cursor_x,y, 1,FONT_HEIGHT), fg );
							}
						}
						x += w as u32;
						},
					LineEnt::FgCol(col) => fg = col,
					LineEnt::BgCol(col) => bg = col,
					}
				}
				// Clear the rest of the line with the current background colour
				surface.fill_rect( Rect::new(x,y, !0, FONT_HEIGHT), bg );

				// Show cursor if line is the active line
				if let Some(cursor_x) = cursor_x {
					if x <= cursor_x {
						surface.fill_rect( Rect::new(cursor_x,y, 1,FONT_HEIGHT), fg );
					}
				}
				// TODO: If this line changed rendered height, set `force` so the entire window scrolls
			}
		}
	}
	fn with_element_at_pos(&self, pos: ::wtk::geom::PxPos, _dims: ::wtk::geom::PxDims, f: ::wtk::WithEleAtPosCb) -> bool {
		f(self, pos)
	}
}
