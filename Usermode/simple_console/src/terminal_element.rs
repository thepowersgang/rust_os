// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// terminal_element.rs
//! Text terminal as a WTK element
use wtk::Colour;
use wtk::geom::Rect;
use std::cell::RefCell;

const BACKGROUND_COLOUR: u32 = 0x33_00_00;	// An ochre red

/// Entry in a lien
enum LineEnt<'a> {
	Text(&'a str),
	FgCol(Colour),
	BgCol(Colour),
}


fn is_unicode_private(v: char) -> bool {
	let c = v as u32;
	if 0xE000 <= c && c <= 0xF8FF {
		true
	}
	else if 0xF0000 <= c && c <= 0xFFFFD {
		true
	}
	else if 0x100000 <= c && c <= 0x10FFFD {
		true
	}
	else {
		false
	}
}

/// NOTE: This `String` stores foreground/background colours using the unicode private codepoint ranges
struct Line(String, ::std::cell::Cell<bool>);
impl Line {
	fn new() -> Line {
		Line(String::new(), Default::default())
	}
	fn is_dirty(&self) -> bool {
		let rv = self.1.get();
		self.1.set(false);
		rv
	}

	fn append_text(&mut self, text: &str) {
		// TODO: Ensure that string doesn't contain special characters that we use as escape codes
		self.0.push_str( text );
		self.1.set(true);
	}
	fn append_fg(&mut self, col: Colour) {
		let col12 = col24_to_12(col);
		let v = 0xF0000 + ((col12 as u32) << 4);
		self.0.push( ::std::char::from_u32(v).expect("Invalid unicode") );
		self.1.set(true);
	}
	fn delete_cell(&mut self) {
		while let Some(v) = self.0.pop()
		{
			use ::UnicodeCombining;
			if v.is_combining() {
				// Combining character, ignore
			}
			else if is_unicode_private(v) {
				// Private (aka contro), ignore
			}
			else {
				// Anything else, this is what we wanted to delete
				break ;
			}
		}
		self.1.set(true);
	}

	fn segs(&self, ofs: usize) -> LineEnts {
		let mut rv = LineEnts {
			string: &self.0,
			iter: self.0.char_indices().peekable(),
			cur_pos: 0,
			};
		while rv.cur_pos < ofs {
			rv.cur_pos = rv.iter.next().expect("Line::segs - offset out of range").0;
		}
		rv
	}
	fn num_cells(&self) -> usize {
		use ::UnicodeCombining;
		let mut rv = 0;
		for seg in self.segs(0)
		{
			match seg
			{
			LineEnt::Text(s) => {
				rv += s.chars().filter(|c| !c.is_combining()).count();
				},
			_ => {},
			}
		}
		rv
	}
}

fn col12_to_24(v12: u16) -> Colour {
	let r4 = ((v12 & 0xF00) >> 8) as u32;
	let g4 = ((v12 & 0x0F0) >> 4) as u32;
	let b4 = ((v12 & 0x00F) >> 0) as u32;
	let v24 = (r4 << 20 | r4 << 16) | (g4 << 12 | g4 << 8) | (b4 << 4 | b4 << 0);
	//kernel_log!("col12_to_24(0x{:03x}) = 0x{:06x}", v12, v24);
	Colour::from_argb32(v24)
}
fn col24_to_12(col: Colour) -> u16 {
	let v24 = col.as_argb32() & 0xFFFFFF;
	let r8 = (v24 & 0xFF0000) >> 16;
	let g8 = (v24 & 0x00FF00) >> 8;
	let b8 = (v24 & 0x0000FF) >> 0;
	let r4 = (r8) >> 4;	assert!(r4 <= 0xF);
	let g4 = (g8) >> 4;	assert!(g4 <= 0xF);
	let b4 = (b8) >> 4;	assert!(b4 <= 0xF);
	let v12 = (r4 << 8 | g4 << 4 | b4 << 0) as u16;
	//kernel_log!("col24_to_12(0x{:06x}) = 0x{:03x}", v24, v12);
	v12
}

struct LineEnts<'a> {
	string: &'a str,
	iter: ::std::iter::Peekable<::std::str::CharIndices<'a>>,
	cur_pos: usize,
}
impl<'a> LineEnts<'a> {
	//fn get_pos(&self) -> usize {
	//	self.cur_pos
	//}
}
impl<'a> Iterator for LineEnts<'a> {
	type Item = LineEnt<'a>;
	fn next(&mut self) -> Option<LineEnt<'a>> {
		let start = self.cur_pos;
		while let Some( &(pos, ch) ) = self.iter.peek()
		{
			self.cur_pos = pos + ch.len_utf8();
			let c = ch as u32;
			if 0xE000 <= c && c <= 0xF8FF {
				if start < pos {
					break ;
				}
				self.iter.next();
				// BMP Private Area - General control
				panic!("LineEnts - Control characters");
			}
			else if 0xF0000 <= c && c <= 0xFFFFD {
				if start < pos {
					break ;
				}
				self.iter.next();
				// Private Use Plane (#15) - Foreground
				let colid = (c - 0xF0000) >> 4;
				return Some(LineEnt::FgCol( col12_to_24(colid as u16) ));
			}
			else if 0x100000 <= c && c <= 0x10FFFD {
				if start < pos {
					break ;
				}
				self.iter.next();
				// Private Use Plane (#16) - Background
				let colid = (c - 0x100000) >> 4;
				return Some(LineEnt::BgCol( col12_to_24(colid as u16) ));
			}
			else {
				// Standard character, eat and keep going
				self.iter.next();
			}
		}
		if start != self.cur_pos {
			Some( LineEnt::Text(&self.string[start .. self.cur_pos]) )
		}
		else {
			None
		}
	}
}

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
pub struct TerminalElementInner
{
	lines: RefCell<Lines>,
	render_cache: RefCell<RenderCache>,
	//render_base: usize,	// Lowest line to render
}
#[derive(Default)]
struct Lines {
	lines: Vec<Line>,
	active_line: usize,
	//cursor_cell: usize,
}
#[derive(Default)]
struct RenderCache {
	line_count: usize,
}

impl<EventCb> TerminalElement<EventCb>
where
	EventCb: FnMut(&mut dyn wtk::WindowTrait, &TerminalElementInner, ::syscalls::gui::Event)
{
	pub fn new(cb: EventCb) -> TerminalElement<EventCb> {
		TerminalElement {
			inner: TerminalElementInner {
				lines: Default::default(),
				render_cache: Default::default(),
				//render_base: 0,
				},
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
		//(cb)(win, self, ev);
		(&mut *cb)(win, &self.inner, ev);
		true
	}
	fn resize(&self, _w: u32, _h: u32) {
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, mut force: bool) {
		
		const FONT_HEIGHT: u32 = 16;
		const FONT_WIDTH : u32 = 8;

		let mut rc_h = self.inner.render_cache.borrow_mut();
		let lines_h = self.inner.lines.borrow();

		force |= rc_h.line_count != lines_h.lines.len();

		rc_h.line_count = lines_h.lines.len();
	
		//let dest_line = self.render_base;
		let mut dest_line = surface.height() / FONT_HEIGHT;
		let line_width = surface.width() / FONT_WIDTH;
		for (idx, line) in lines_h.lines.iter().enumerate().rev()
		{
			if line.is_dirty() || force
			{
				let num_lines = (line.num_cells() as u32 + line_width - 1) / line_width;
				dest_line -= num_lines;
				let mut x = 0;
				let y = dest_line * FONT_HEIGHT;

				surface.fill_rect( Rect::new(x,y, !0,num_lines*FONT_HEIGHT), Colour::from_argb32(BACKGROUND_COLOUR) );

				let mut fg = Colour::from_argb32(0xFFFFFF);
				for seg in line.segs(0)
				{
					match seg
					{
					LineEnt::Text(text) => {
						let w = surface.draw_text( Rect::new(x,y, !0,!0), text.chars(), fg );
						x += w as u32;
						},
					LineEnt::FgCol(col) => fg = col,
					LineEnt::BgCol(_col) => {},
					}
				}

				// Show cursor if line is the active line
				if idx == lines_h.active_line {
					surface.fill_rect( Rect::new(x,y, 1,FONT_HEIGHT), fg );
				}
			}
		}
	}
	fn with_element_at_pos(&self, pos: ::wtk::geom::PxPos, _dims: ::wtk::geom::PxDims, f: ::wtk::WithEleAtPosCb) -> bool { f(self, pos) }
}

// Terminal interface
impl ::Terminal for TerminalElementInner
{
	fn set_foreground(&self, col: Colour) {
		//self.cur_fg = col;
		self.lines.borrow_mut().cur_mut().append_fg(col);
	}
	
	fn flush(&self) {
		// Does nothing
	}

	fn cur_col(&self) -> usize {
		self.lines.borrow().cur_col()
	}

	fn delete_left(&self) {
		//if self.cur_col > 0 {
		//	self.cursor_left();
		//	self.shift_line_left();
		//}
		
		self.lines.borrow_mut().cur_mut().delete_cell();
	}
	fn delete_right(&self) {
		//self.shift_line_left();
	}

	fn cursor_left(&self) {
		panic!("TODO: TerminalElement::cursor_left");
		//assert!(self.cur_col != 0);
		//self.cur_col -= 1;
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
		let mut buf_lines = self.lines.borrow_mut();
		let mut lines_iter = s.split('\n');
		buf_lines.cur_mut().append_text( lines_iter.next().unwrap() );
		for line in lines_iter
		{
			buf_lines.active_line += 1;
			buf_lines.cur_mut().append_text(line);
		}
	}
}

impl Lines
{
	fn cur_mut(&mut self) -> &mut Line {
		if self.active_line == self.lines.len() {
			self.lines.push(Line::new());
		}
		&mut self.lines[self.active_line]
	}

	fn cur_col(&self) -> usize {
		self.lines.get(self.active_line).map(|l| l.num_cells()).unwrap_or(0)
	}
}

