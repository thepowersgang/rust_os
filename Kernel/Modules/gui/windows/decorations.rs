//
//
//
//! 
use super::winbuf::WinBuf;
use ::Rect;

pub struct DecorTemplate<T: AsRef<[u32]>> {
	/// Total width
	w: u32,
	/// Total width
	h: u32,
	/// Width of the lefthand fixed region
	left: u32,
	/// Height of the top fixed region
	top: u32,
	/// Image dat
	data: T,
}

/// Used to coerce
const fn as_slice<'a,T>(v: &'a [T]) -> &'a [T] { v }
pub static WINDOW_TEMPLATE: DecorTemplate<&'static [u32]> = DecorTemplate {
	w: 3, h: (1+16+1)+1+1,
	left: 1, top: (1+16+1),
	data: as_slice(&[
		// Top fixed region
		0xFFFFFF, 0xFFFFFF, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0x000000, 0xFFFFFF,
		0xFFFFFF, 0xFFFFFF, 0xFFFFFF,

		// Middle variable
		0xFFFFFF, 0xFF_000000, 0xFFFFFF,

		// Bottom fixed
		0xFFFFFF, 0xFFFFFF, 0xFFFFFF,
		]),
	};

impl<T: AsRef<[u32]>> DecorTemplate<T>
{
	pub fn render(&self, buf: &WinBuf, rect: Rect)
	{
		let dst_bottom = (rect.h() - self.bottom()) as usize;
		for row in 0 .. self.top as usize {
			self.render_line( row, buf.scanline_rgn_mut(rect.top() as usize + row, rect.left() as usize, rect.w() as usize) );
		}
		for row in self.top as usize .. dst_bottom {
			self.render_line( self.top as usize, buf.scanline_rgn_mut(rect.top() as usize + row, rect.left() as usize, rect.w() as usize) );
		}
		for i in 0 .. self.bottom() as usize {
			self.render_line( self.top as usize + 1 + i, buf.scanline_rgn_mut(rect.top() as usize + dst_bottom + i, rect.left() as usize, rect.w() as usize) );
		}
	}

	fn render_line(&self, sline: usize, dst: &mut [u32]) {
		let src = &self.data.as_ref()[self.w as usize * sline .. ][ .. self.w as usize];
		let dst_w = dst.len();
		let dst_right = dst_w - self.right() as usize;

		for col in 0 .. self.left as usize {
			dst[col] = src[col];
		}

		// if mid_c's top byte is 0xFF, it's fully transparent
		let mid_c = src[self.left as usize];
		if mid_c >> 24 != 0xFF
		{
			for col in self.left as usize .. dst_right {
				dst[col] = mid_c;
			}
		}

		for i in 0 .. self.right() as usize {
			dst[dst_right + i] = src[self.left as usize + 1 + i];
		}
	}
	pub fn fixed_width(&self) -> u32 {
		self.w - 1
	}
	pub fn left(&self) -> u32 {
		self.left
	}
	pub fn right(&self) -> u32 {
		self.w - self.left - 1
	}
	pub fn fixed_height(&self) -> u32 {
		self.h - 1
	}
	pub fn top(&self) -> u32 {
		self.top
	}
	pub fn bottom(&self) -> u32 {
		self.h - self.top - 1
	}
}


