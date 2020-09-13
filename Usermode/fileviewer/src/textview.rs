// Tifflin OS File Viewer
// - By John Hodge (thePowersGang)
//
// fileviewer/src/textview.rs
//! Text buffer viewer widget
//!
//! 
use wtk::geom::Rect;
use wtk::Colour;

pub struct Widget
{
	visible_line_count: ::std::cell::Cell<usize>,
	first_line: usize,

	lines: ::std::cell::RefCell< Vec<Line> >,
}

struct Line
{
	file_offset: u64,
	//file_size: usize,	// May be != data.len() if the line wasn't valid UTF-8
	// TODO: Use ByteString or other - and do box-chars for invalid codepoints?
	data: String,
}


impl Widget
{
	pub fn new() -> Widget {
		Widget {
			visible_line_count: Default::default(),
			first_line: 0,
			lines: Default::default(),
			}
	}

	/// Populate the buffer with the provided "file"
	pub fn populate<R>(&self, mut file: R) -> ::std::io::Result<()>
	where
		R: ::std::io::Read + ::std::io::Seek
	{
		let line_count = self.visible_line_count.get();
		let cached_line_count = line_count + self.first_line;
		let mut lines = self.lines.borrow_mut();

		if cached_line_count > lines.len() {
			let offset = if lines.len() == 0 {
					assert!( self.first_line == 0 );
					0
				}
				else {
					lines[lines.len()-1].file_offset
				};
			file.seek( ::std::io::SeekFrom::Start(offset) )?;
			while lines.len() < cached_line_count
			{
				lines.push( Line::new(&mut file)? );
			}
		}
		else {
			//self.lines.truncate( cached_line_count );
		}

		Ok( () )
	}

	fn line_height(&self) -> u32 {
		16
	}
}
impl ::wtk::Element for Widget
{
	fn resize(&self, _w: u32, h: u32) {
		let line_count = ((h + self.line_height() - 1) / self.line_height()) as usize;
		self.visible_line_count.set( line_count );
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
		if force
		{
			surface.fill_rect(Rect::new(0,0, !0,!0), Colour::theme_text_bg());
			for (idx, line) in Iterator::enumerate( self.lines.borrow()[self.first_line..][..self.visible_line_count.get()].iter() )
			{
				surface.draw_text(
					Rect::new(0, idx as u32 * self.line_height(),  !0, !0),
					line.data.chars(),
					Colour::theme_text()
					);
			}
		}
	}
	fn with_element_at_pos(&self, pos: ::wtk::geom::PxPos, _dims: ::wtk::geom::PxDims, f: ::wtk::WithEleAtPosCb) -> bool {
		f(self, pos)
	}
}

impl Line
{
	fn new<R>(mut file: R) -> ::std::io::Result<Self>
	where
		R: ::std::io::Read + ::std::io::Seek
	{
		let start = file.seek( ::std::io::SeekFrom::Current(0) )?;
		let bytes = {
			let mut bytes = Vec::new();
			let mut b = [0];
			while file.read(&mut b)? == 1 && b[0] != b'\n' {
				bytes.push( b[0] );
			}
			bytes
			};

		//let byte_len = bytes.len();
		let s = match String::from_utf8(bytes)
			{
			Ok(v) => v,
			Err(e) => {
				let valid_len = e.utf8_error().valid_up_to();
				let mut b = e.into_bytes();
				b.truncate(valid_len);
				b.push( b'?' );
				// SAFE: Valid length
				unsafe { String::from_utf8_unchecked(b) }
				},
			};
		Ok(Line {
			file_offset: start,
			//file_size: byte_len,
			data: s,
			})
	}
}

