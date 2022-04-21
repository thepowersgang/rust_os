// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video/bootvideo.rs
//! Early-boot video support (using bootloader-provided framebuffer)
#[allow(unused_imports)]
use crate::prelude::*;
use super::{Dims, Rect, Pos};

/// Bit format of the linear framebuffer, if interpreted as a n-bit little-endian number
#[derive(Copy,Clone,Debug)]
pub enum VideoFormat
{
	/// ARGB (B, G, R, x)
	X8R8G8B8,
	/// BGRX (x, R, G, B)
	B8G8R8X8,
	/// 24-bit RGB (B, G, R)
	R8G8B8,
	B8G8R8,
	/// 16-bit RGB (B g, g R)
	R5G6B5,
}

fn round_from_8(val: u32, bits: usize) -> u32 {
	let rv = val >> (8 - bits);
	if rv != (0xFF >> (8-bits)) && (val >> (8 - bits - 1)) & 1 == 1 {
		rv + 1
	}
	else {
		rv
	}
}
impl VideoFormat
{
	pub fn col_from_xrgb(&self, src: u32) -> u32 {
		let (r,g,b) = ( (src >> 16) & 0xFF, (src >> 8) & 0xFF, src & 0xFF );
		match *self
		{
		VideoFormat::X8R8G8B8 => src & 0xFFFFFF,
		VideoFormat::R5G6B5 => (round_from_8(r, 5) << 11) | (round_from_8(g, 6) << 5) | (round_from_8(b, 5) << 0),
		_ => todo!("col_from_xrgb"),
		}
	}

	pub fn bytes_per_pixel(&self) -> usize {
		match self
		{
		&VideoFormat::X8R8G8B8 => 4,
		&VideoFormat::R5G6B5 => 2,
		fmt @ _ => todo!("VideoFormat::bytes_per_pixel - VideoFormat::{:?}", fmt),
		}
	}
}

/// Representation of the video mode
#[derive(Copy,Clone)]
pub struct VideoMode
{
	pub width: u16,
	pub height: u16,
	pub fmt: VideoFormat,
	pub pitch: usize,
	pub base: crate::arch::memory::PAddr,
}
impl_fmt!{
	Debug(self,f) for VideoMode {
		write!(f, "VideoMode {{ {}x{} {:?} {}b @ {:#x} }}",
			self.width, self.height, self.fmt, self.pitch, self.base)
	}
}

/// Framebuffer for the boot video system
pub struct Framebuffer
{
	buffer: Buffer,
	cursor_pos: Option<Pos>,
	cursor_cache: Vec<u8>,
	cursor_data: CursorData,
}

struct CursorData
{
	dims: Dims,
	data: Vec<u8>,	// Bitmap : 1,1 alpha and colour
}
struct Buffer
{
	mode: VideoMode,
	buffer: crate::memory::virt::AllocHandle,
}

impl Framebuffer
{
	/// Create a new `Framebuffer`
	pub fn new(mode: VideoMode) -> Framebuffer
	{
		// 1. Some quick sanity checks
		let exp_pitch = match mode.fmt
			{
			VideoFormat::X8R8G8B8 => mode.width as usize * 4,
			VideoFormat::R8G8B8 => mode.width as usize * 3, // 24 bpp
			VideoFormat::R5G6B5 => mode.width as usize * 2,	// 16 bpp
			_ => todo!("Framebuffer::new: Handle format {:?}", mode.fmt),
			};
		assert!(mode.pitch >= exp_pitch, "Framebuffer::new: Pitch {:#x} is not sane, exp >= {:#x}", mode.pitch, exp_pitch);
		
		let fb_size = (mode.base as usize % crate::PAGE_SIZE) + mode.pitch * mode.height as usize;
		let n_pages = (fb_size + crate::PAGE_SIZE - 1) / crate::PAGE_SIZE;
		// Assuming SAFE: The framebuffer shouldn't be multiple mapped
		let alloc = match unsafe { crate::memory::virt::map_hw_rw( mode.base, n_pages, module_path!() ) }
			{
			Ok(v) => v,
			Err(e) => panic!("Failed to map boot framebuffer {:#x} {}pg - {}",
				mode.base, n_pages, e),
			};
		Framebuffer {
			buffer: Buffer{
				mode: mode,
				buffer: alloc,
				},
			cursor_cache: Vec::new(),
			cursor_pos: None,
			cursor_data: CursorData::new(),
		}
	}
}

impl Buffer
{
	/// Obtain the framebuffer as a byte slice
	fn buffer(&mut self) -> &mut [u8] {
		self.buffer.as_mut_slice(
			self.mode.base as usize % crate::PAGE_SIZE,
			self.mode.pitch * self.mode.height as usize
			)
	}
	fn scanline(&mut self, line: usize) -> &mut [u8] {
		assert!(line < self.mode.height as usize);
		let pitch = self.mode.pitch;
		let ofs = line * pitch;
		let fb = self.buffer();
		&mut fb[ofs .. ofs + pitch]
	}

	fn scanline_slice(&mut self, line: usize, start: usize, end: usize) -> &mut [u8] {
		assert!(start <= end, "scanline_slice - start {} > end {}", start, end);
		assert!(end <= self.mode.width as usize, "scanline_slice - end {} > width {}", end, self.mode.width);

		let bytes_per_pixel = self.mode.fmt.bytes_per_pixel();
		&mut self.scanline(line)[start * bytes_per_pixel .. end * bytes_per_pixel]
	}
}

impl super::Framebuffer for Framebuffer
{
	fn as_any(&self) -> &dyn (::core::any::Any) {
		self
	}
	fn activate(&mut self) {
		// no-op, already active
	}
	
	fn get_size(&self) -> Dims {
		Dims::new(self.buffer.mode.width as u32, self.buffer.mode.height as u32)
	}
	fn set_size(&mut self, _newsize: Dims) -> bool {
		false
	}
	
	fn blit_inner(&mut self, dst: Rect, src: Rect) {
		if dst.dims() != src.dims() {
			return ;
		}
		//let redraw_cursor = self.clobber_cursor(dst) || self.clobber_cursor(src);
		todo!("Framebuffer::blit_inner");
	}
	fn blit_ext(&mut self, _dst: Rect, _src: Rect, _srf: &dyn super::Framebuffer) -> bool {
		false
	}
	fn blit_buf(&mut self, dst: Rect, buf: super::StrideBuf<'_, u32>) {
		let redraw_cursor = self.clobber_cursor(dst);

		//log_trace!("Framebuffer::blit_buf(dst={})", dst);
		let output_fmt = self.buffer.mode.fmt;
		let src_pitch = dst.w() as usize;
		
		assert!(dst.left()  <  self.buffer.mode.width as u32);
		assert!(dst.right() <= self.buffer.mode.width as u32);
		assert!(dst.top()    <  self.buffer.mode.height as u32);
		assert!(dst.bottom() <= self.buffer.mode.height as u32);
		
		assert!(buf.is_round(src_pitch));

		let bpp = output_fmt.bytes_per_pixel();
		// Iterate across destination row nums and source rows
		//for (row,src) in Iterator::zip( dst.top() .. dst.bottom(), buf.chunks(src_pitch) )
		match output_fmt
		{
		VideoFormat::X8R8G8B8 => {
			assert!(bpp == 4);
			// This mode corresponds to the internal format, so can use fast operations (raw byte copy)
			for (row,src) in crate::lib::ExactZip::new( dst.top() .. dst.bottom(), buf.chunks(src_pitch) )
			{
				let seg = self.buffer.scanline_slice(row as usize, dst.left() as usize, dst.right() as usize);
				seg.copy_from_slice( crate::lib::as_byte_slice(src) );
			}
			},
		VideoFormat::R5G6B5 =>
			for (row,src) in crate::lib::ExactZip::new( dst.top() .. dst.bottom(), buf.chunks(src_pitch) )
			{
				let seg = self.buffer.scanline_slice(row as usize, dst.left() as usize, dst.right() as usize);
				for (px,&col) in crate::lib::ExactZip::new( seg.chunks_mut(bpp), src.iter() )
				{
					let col16 = output_fmt.col_from_xrgb(col);
					px[0] = ((col16 >>  0) & 0xFF) as u8;
					px[1] = ((col16 >>  8) & 0xFF) as u8;
				}
			},
		fmt @ _ => todo!("Framebuffer::blit_buf - {:?}", fmt),
		}

		if redraw_cursor {
			self.render_cursor();
		}
	}
	fn fill(&mut self, dst: Rect, colour: u32) {
		let redraw_cursor = self.clobber_cursor(dst);

		let output_fmt = self.buffer.mode.fmt;
		assert!(dst.left()  <  self.buffer.mode.width as u32);
		assert!(dst.right() <= self.buffer.mode.width as u32);
		assert!(dst.top()    <  self.buffer.mode.height as u32);
		assert!(dst.bottom() <= self.buffer.mode.height as u32);
		
		let bpp = output_fmt.bytes_per_pixel();
		for row in dst.top() .. dst.bottom()
		{
			let seg = self.buffer.scanline_slice(row as usize, dst.left() as usize, dst.right() as usize);
			match output_fmt
			{
			VideoFormat::X8R8G8B8 => {
				for px in seg.chunks_mut(bpp)
				{
					px[0] = ((colour >>  0) & 0xFF) as u8;
					px[1] = ((colour >>  8) & 0xFF) as u8;
					px[2] = ((colour >> 16) & 0xFF) as u8;
					//px[3] = ((col >> 32) & 0xFF) as u8;
				}
				},
			VideoFormat::R5G6B5 => {
				for px in seg.chunks_mut(bpp)
				{
					let col16 = output_fmt.col_from_xrgb(colour);
					px[0] = ((col16 >>  0) & 0xFF) as u8;
					px[1] = ((col16 >>  8) & 0xFF) as u8;
				}
				},
			fmt @ _ => todo!("Framebuffer::blit_buf - {:?}", fmt),
			}
		}

		if redraw_cursor {
			self.render_cursor();
		}
	}

	fn move_cursor(&mut self, p: Option<Pos>) {
		if p != self.cursor_pos
		{
			// 1. Un-render the cursor
			self.unrender_cursor();
			// 2. Update position
			self.cursor_pos = p;
			// 3. Re-render if required
			self.render_cursor();
		}
	}
}

impl Framebuffer
{
	fn cursor_rect(&self) -> Option<Rect> {
		self.cursor_pos.map(|p| Rect::new_pd(p, self.cursor_data.dims))
	}
	/// Returns true if modifying the provided rect will clobber the cursor
	///
	/// NOTE: Also hides the cursor in preparation for update
	fn clobber_cursor(&mut self, rect: Rect) -> bool {
		let clobbered = self.cursor_rect().and_then(|r| r.intersect(&rect)).is_some();

		if clobbered {
			self.unrender_cursor();
		}
		clobbered
	}
	fn unrender_cursor(&mut self) {
		if let Some(r) = self.cursor_rect()
		{
			let bpp = self.buffer.mode.fmt.bytes_per_pixel();
			
			for (src, row) in Iterator::zip( self.cursor_cache.chunks(self.cursor_data.dims.w as usize * bpp),  r.top() .. r.bottom() )
			{
				if row >= self.buffer.mode.height as u32 {
					break ;
				}
				let right = ::core::cmp::min(self.buffer.mode.width as usize, r.right() as usize);
				let seg = self.buffer.scanline_slice(row as usize, r.left() as usize, right);
				let src = &src[..seg.len()];
				seg.clone_from_slice(src);
			}
		}
	}

	fn render_cursor(&mut self)
	{
		if let Some(r) = self.cursor_rect()
		{
			let output_fmt = self.buffer.mode.fmt;
			let bpp = output_fmt.bytes_per_pixel();
			
			// 1. Save the area of the screen underneath the cursor
			// 2. Render the cursor over this area
			self.cursor_cache.resize( r.h() as usize * r.w() as usize * bpp, 0 );
			for (dst, row) in Iterator::zip( self.cursor_cache.chunks_mut(self.cursor_data.dims.w as usize * bpp),  r.top() .. r.bottom() )
			{
				if row >= self.buffer.mode.height as u32 {
					break ;
				}
				let right = ::core::cmp::min(self.buffer.mode.width as usize, r.right() as usize);
				let seg = self.buffer.scanline_slice(row as usize, r.left() as usize, right);
				// - Handle the case where the cursor is off the RHS
				dst[..seg.len()].clone_from_slice( seg );
				self.cursor_data.render_line( (row - r.top()) as usize, seg, output_fmt );
			}
		}
	}
}

impl CursorData
{
	fn new() -> CursorData {
		CursorData {
			dims: Dims::new(8,16),
			data: vec![
				0x7F,0x00,0x00,0x00, 0x00,0x00,0x00,0x00,
				0x7F,0x7F,0x00,0x00, 0x00,0x00,0x00,0x00,
				0x7F,0xFF,0x7F,0x00, 0x00,0x00,0x00,0x00,
				0x7F,0xFF,0xFF,0x7F, 0x00,0x00,0x00,0x00,
				0x7F,0xFF,0xFF,0xFF, 0x7F,0x00,0x00,0x00,
				0x7F,0xFF,0xFF,0xFF, 0xFF,0x7F,0x00,0x00,
				0x7F,0xFF,0xFF,0xFF, 0xFF,0xFF,0x7F,0x00,
				0x7F,0xFF,0xFF,0xFF, 0xFF,0x7F,0x7F,0x7F,

				0x7F,0xFF,0xFF,0xFF, 0x7F,0x00,0x00,0x00,
				0x7F,0xFF,0x7F,0xFF, 0x7F,0x00,0x00,0x00,
				0x7F,0x7F,0x00,0x7F, 0xFF,0x7F,0x00,0x00,
				0x00,0x00,0x00,0x7F, 0xFF,0x7F,0x00,0x00,
				0x00,0x00,0x00,0x7F, 0xFF,0x7F,0x00,0x00,
				0x00,0x00,0x00,0x7F, 0xFF,0xFF,0x7F,0x00,
				0x00,0x00,0x00,0x00, 0x7F,0xFF,0x7F,0x00,
				0x00,0x00,0x00,0x00, 0x7F,0x7F,0x7F,0x00
				],
			}
	}

	fn render_line(&self, row: usize, dst: &mut [u8], format: VideoFormat)
	{
		let pitch = self.dims.w as usize;
		assert!(row * pitch < self.data.len());
		let data = &self.data[row * pitch ..][.. pitch];

		let bpp = format.bytes_per_pixel();
		match format
		{
		VideoFormat::X8R8G8B8 => {
			for (px, &val) in crate::lib::ExactZip::new( dst.chunks_mut(bpp), data.iter() )
			{
				if let Some(colour) = self.get_colour(val)
				{
					px[0] = ((colour >>  0) & 0xFF) as u8;
					px[1] = ((colour >>  8) & 0xFF) as u8;
					px[2] = ((colour >> 16) & 0xFF) as u8;
					//px[3] = ((col >> 32) & 0xFF) as u8;
				}
			}
			},
		VideoFormat::R5G6B5 => {
			for (px, &val) in crate::lib::ExactZip::new( dst.chunks_mut(bpp), data.iter() )
			{
				if let Some(colour) = self.get_colour(val)
				{
					let col16 = format.col_from_xrgb(colour);
					px[0] = ((col16 >>  0) & 0xFF) as u8;
					px[1] = ((col16 >>  8) & 0xFF) as u8;
				}
			}
			},
		fmt @ _ => todo!("CursorData::render_line - fmt={:?}", fmt),
		}
	}

	fn get_colour(&self, val: u8) -> Option<u32> {
		let alpha = val & 0x7F;
		if alpha == 0 {
			None
		}
		else {
			if val & 0x80 != 0 {
				Some(!0)
			}
			else {
				Some(0)
			}
		}
	}
}
