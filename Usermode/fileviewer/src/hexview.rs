// Tifflin OS File Viewer
// - By John Hodge (thePowersGang)
//
//! 16-byte hex view
use wtk::geom::Rect;
use wtk::Colour;

pub struct Widget
{
	offset_width: u8,
	//byte_width: u8,
	
	state: ::std::cell::RefCell<State>,
}

#[derive(Default)]
struct State {
	/// Number of lines avaliable in the view
	view_size: usize,
	/// Byte start position of the first line
	view_start: u64,
	/// On-screen data
	data: Vec<u8>,
}

struct Seg<'a>(&'a [u8]);

const CHUNK_SIZE: usize = 16;

impl Widget
{
	pub fn new() -> Widget {
		Widget {
			offset_width: 6,
			//byte_width: 16,
			state: Default::default(),
			}
	}

	/// Update content
	pub fn populate<F: ::std::io::Read+::std::io::Seek>(&self, mut f: F) -> ::std::io::Result<()> {
		let mut st = self.state.borrow_mut();
		st.view_start = f.seek(::std::io::SeekFrom::Current(0))?;
		//st.data = Vec::with_capacity( CHUNK_SIZE * self.view_size );
		st.data.clear();
		for i in 0 .. st.view_size
		{
			let ofs = i * CHUNK_SIZE;
			st.data.extend( ::std::iter::repeat(0).take(CHUNK_SIZE) );
			let count = f.read(&mut st.data[ofs ..])?;
			if count < CHUNK_SIZE {
				st.data.truncate(ofs + count);
				break ;
			}
		}
		Ok( () )
	}
	pub fn get_start(&self) -> u64 {
		self.state.borrow().view_start
	}
	pub fn get_capacity(&self) -> usize {
		self.state.borrow().view_size
	}
	pub fn min_width(&self) -> u32 {
		(self.offset_width as u32 + (3*8) + 1 + (3*8) + 2 + 8 + 1 + 8) * 8
	}

	fn line_height(&self) -> u32 {
		16
	}
}
impl ::wtk::Element for Widget
{
	fn resize(&self, _w: u32, h: u32) {
		self.state.borrow_mut().view_size = ((h + self.line_height()-1) / self.line_height()) as usize;
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool)
	{
		if force
		{
			surface.fill_rect(Rect::new(0,0, !0,!0), Colour::theme_text_bg());
			let state = self.state.borrow();
			for (idx, bytes) in state.data.chunks(16).enumerate()
			{
				use std::fmt::Write;
				let ofs = state.view_start + idx as u64 * 16;
				let seg1 = if bytes.len() >= 8 { Seg(&bytes[..8]) } else { Seg(bytes) };
				let seg2 = if bytes.len() >= 8 { Seg(&bytes[8..]) } else { Seg(&[]) };
				
				let _ = write!(
					surface.draw_text_fmt(Rect::new(0, idx as u32 * self.line_height(),  !0,!0), Colour::theme_text()),
					"{:0ofs_width$x} {:x}  {:x}  {}{}",
					ofs,
					seg1, seg2,  seg1, seg2,
					// 5- Params
					ofs_width=self.offset_width as usize
					);
			}
		}
	}
	fn with_element_at_pos(&self, pos: ::wtk::geom::PxPos, _dims: ::wtk::geom::PxDims, f: ::wtk::WithEleAtPosCb) -> bool {
		f(self, pos)
	}
}

impl<'a> ::std::fmt::LowerHex for Seg<'a>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		for i in 0 .. 8 {
			if i > 0 {
				f.write_str(" ")?;
			}
			if let Some(e) = self.0.get(i) {
				write!(f, "{:02x}", *e)?;
			}
			else {
				f.write_str("  ")?;
			}
		}
		Ok( () )
	}
}
impl<'a> ::std::fmt::Display for Seg<'a>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		for i in 0 .. 8 {
			if let Some(&v) = self.0.get(i) {
				if 0x20 <= v && v <= 0x7F {
					::std::fmt::Write::write_char(f, v as char)?;
				}
				else {
					f.write_str(".")?;
				}
			}
			else {
				f.write_str(" ")?;
			}
		}
		Ok( () )
	}
}

