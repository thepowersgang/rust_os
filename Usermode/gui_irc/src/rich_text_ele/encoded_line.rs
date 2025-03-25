//! 
use ::wtk::Colour;


/// NOTE: This `String` stores foreground/background colours using the unicode private codepoint ranges
#[derive(Default)]
pub struct Line {
	data: String,
	is_dirty: ::std::cell::Cell<bool>,
}

/// A decoded line entry
pub enum LineEnt<'a> {
	Text(&'a str),
	FgCol(Colour),
	BgCol(Colour),
}

impl Line {
	pub fn is_dirty(&self) -> bool {
		self.is_dirty.replace(false)
	}

	pub fn append_text(&mut self, text: &str) {
		// TODO: Ensure that string doesn't contain special characters that we use as escape codes
		self.data.push_str( text );
		self.is_dirty.set(true);
	}
	pub fn append_fg(&mut self, col: Colour) {
		self.data.push( CodepointClass::PrivateUse15(col24_to_12(col) << 4).to_char() );
		self.is_dirty.set(true);
	}
	pub fn append_bg(&mut self, col: Colour) {
		self.data.push( CodepointClass::PrivateUse16(col24_to_12(col) << 4).to_char() );
		self.is_dirty.set(true);
	}
	pub fn delete_cell_back(&mut self)
	{
		while let Some(v) = self.data.pop()
		{
			use ::wtk::surface::UnicodeCombining;
			if let CodepointClass::Allocated(_) = CodepointClass::from_char(v) {
				if !v.is_combining() {
					// Allocated non-combining character, this is what we wanted to delete
					break;
				}
				else {
					// Combining character, erase through this
				}
			}
			else {
				// Control character (colours), erase through
			}
		}
		self.is_dirty.set(true);
	}

	pub fn segs(&self, ofs: usize) -> LineEnts {
		let mut rv = LineEnts {
			string: &self.data,
			iter: self.data.char_indices().peekable(),
			cur_pos: 0,
			};
		while rv.cur_pos < ofs {
			rv.cur_pos = rv.iter.next().expect("Line::segs - offset out of range").0;
		}
		rv
	}
	pub fn num_cells(&self) -> usize {
		let mut rv = 0;
		for seg in self.segs(0)
		{
			match seg
			{
			LineEnt::Text(s) => {
				rv += rendered_cell_count(s);
				},
			_ => {},
			}
		}
		rv
	}
}

pub fn rendered_cell_count(s: &str) -> usize {
	use ::wtk::surface::UnicodeCombining;
	// If the first codepoint is a combining character, then it will be rendered to its own cell (combining with an implicit space)
	let prefix_len = if s.chars().next().map(|c| c.is_combining()).unwrap_or_default() {
		1
	}
	else {
		0
	};
	prefix_len + s.chars().filter(|c| !c.is_combining()).count()
}

/// Iterator over entries in a line
pub struct LineEnts<'a> {
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
			match CodepointClass::from_char(ch)
			{
			// Standard character, eat and keep going
			CodepointClass::Allocated(_) => {
				self.iter.next();
			},
			// BMP Private Area - General control
			CodepointClass::BmpPrivate(_) => {
				if start < pos {
					break ;
				}
				self.iter.next();
				panic!("LineEnts - Control characters");
			},
			// Private Use Plane (#15) - Foreground
			// - This has nearly 16 bits available (it has 0xFFFE entries, not 0x10000)
			CodepointClass::PrivateUse15(val) => {
				if start < pos {
					break ;
				}
				self.iter.next();
				return Some(LineEnt::FgCol( colour_from_encoded(val) ));
			},
			// Private Use Plane (#16) - Background
			CodepointClass::PrivateUse16(val) => {
				if start < pos {
					break ;
				}
				self.iter.next();
				return Some(LineEnt::BgCol( colour_from_encoded(val) ));
			},
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

enum CodepointClass {
	Allocated(char),
	/// Private area in the Basic Multilingual Plane
	/// - 0xE000->0xF8FF - 0x1900 entries
	BmpPrivate(u16),
	/// Plane 15: Private Use - 0xFFFE entries
	PrivateUse15(u16),
	/// Plane 16: Private Use - 0xFFFE entries
	PrivateUse16(u16),
}
impl CodepointClass {
	fn from_char(v: char) -> Self {
		let c = v as u32;
		if 0xE000 <= c && c <= 0xF8FF {
			CodepointClass::BmpPrivate( (c - 0xE000) as u16 )
		}
		else if 0xF0000 <= c && c <= 0xFFFFD {
			CodepointClass::PrivateUse15( (c - 0xF0000) as u16 )
		}
		else if 0x100000 <= c && c <= 0x10FFFD {
			CodepointClass::PrivateUse16( (c - 0x100000) as u16 )
		}
		else {
			CodepointClass::Allocated(v)
		}
	}
	fn to_char(self) -> char {
		match self {
		CodepointClass::Allocated(ch) => ch,
		CodepointClass::BmpPrivate(v) => char::from_u32(0xE000 + v as u32).unwrap(),
		CodepointClass::PrivateUse15(v) => char::from_u32( 0xF0000 + v as u32).unwrap(),
		CodepointClass::PrivateUse16(v) => char::from_u32(0x100000 + v as u32).unwrap(),
		}
	}
}

fn colour_from_encoded(val: u16) -> Colour {
	col12_to_24(val >> 4)
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