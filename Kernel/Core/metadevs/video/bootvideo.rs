use _common::*;
use super::{Dims, Rect};

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
}

/// Representation of the video mode
#[derive(Copy,Clone)]
pub struct VideoMode
{
	pub width: u16,
	pub height: u16,
	pub fmt: VideoFormat,
	pub pitch: usize,
	pub base: ::arch::memory::PAddr,
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
	mode: VideoMode,
	buffer: ::memory::virt::AllocHandle,
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
		
		let fb_size = (mode.base as usize % ::PAGE_SIZE) + mode.pitch * mode.height as usize;
		let n_pages = (fb_size + ::PAGE_SIZE - 1) / ::PAGE_SIZE;
		let alloc = match ::memory::virt::map_hw_rw( mode.base, n_pages, module_path!() )
			{
			Ok(v) => v,
			Err(e) => panic!("Failed to map boot framebuffer {:#x} {}pg - {}",
				mode.base, n_pages, e),
			};
		Framebuffer {
			mode: mode,
			buffer: alloc,
		}
	}
	
	/// Obtain the framebuffer as a byte slice
	fn buffer(&mut self) -> &mut [u8] {
		self.buffer.as_mut_slice(
			self.mode.base as usize % ::PAGE_SIZE,
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
}

impl super::Framebuffer for Framebuffer
{
	fn as_any(&self) -> &::core::any::Any {
		self as &::core::any::Any
	}
	fn activate(&mut self) {
		// no-op, already active
	}
	
	fn get_size(&self) -> Dims {
		Dims::new(self.mode.width as u32, self.mode.height as u32)
	}
	fn set_size(&mut self, _newsize: Dims) -> bool {
		false
	}
	
	fn blit_inner(&mut self, dst: Rect, src: Rect) {
		if dst.dims() != src.dims() {
			return ;
		}
		todo!("Framebuffer::blit_inner");
	}
	fn blit_ext(&mut self, _dst: Rect, _src: Rect, _srf: &super::Framebuffer) -> bool {
		false
	}
	fn blit_buf(&mut self, dst: Rect, buf: &[u32]) {
		//log_trace!("Framebuffer::blit_buf(dst={})", dst);
		let output_fmt = self.mode.fmt;
		let src_pitch = dst.w() as usize;
		
		assert!(dst.left()  <  self.mode.width as u32);
		assert!(dst.right() <= self.mode.width as u32);
		assert!(dst.top()    <  self.mode.height as u32);
		assert!(dst.bottom() <= self.mode.height as u32);
		
		assert!(buf.len() >= src_pitch);
		assert!(buf.len() % src_pitch == 0);
		// Iterate across destination row nums and source rows
		for (row,src) in (dst.top() .. dst.bottom()).zip( buf.chunks(src_pitch) )
		{
			let sl = self.scanline(row as usize);
			match output_fmt
			{
			VideoFormat::X8R8G8B8 => {
				let bpp = 4;
				let left_byte  = dst.left()  as usize * bpp;
				let right_byte = dst.right() as usize * bpp;
				let seg = &mut sl[left_byte .. right_byte];
				for (px,col) in seg.chunks_mut(bpp).zip( src.iter().cloned() )
				{
					px[0] = ((col >>  0) & 0xFF) as u8;
					px[1] = ((col >>  8) & 0xFF) as u8;
					px[2] = ((col >> 16) & 0xFF) as u8;
					//px[3] = ((col >> 32) & 0xFF) as u8;
				}
				},
			VideoFormat::R5G6B5 => {
				let bpp = 2;
				let left_byte  = dst.left()  as usize * bpp;
				let right_byte = dst.right() as usize * bpp;
				let seg = &mut sl[left_byte .. right_byte];
				for (px,col) in seg.chunks_mut(bpp).zip( src.iter().cloned() )
				{
					let col16 = output_fmt.col_from_xrgb(col);
					px[0] = ((col16 >>  0) & 0xFF) as u8;
					px[1] = ((col16 >>  8) & 0xFF) as u8;
				}
				},
			fmt @ _ => todo!("Framebuffer::blit_buf - {:?}", fmt),
			}
		}
	}
	fn fill(&mut self, dst: Rect, colour: u32) {
		let output_fmt = self.mode.fmt;
		assert!(dst.left()  <  self.mode.width as u32);
		assert!(dst.right() <= self.mode.width as u32);
		assert!(dst.top()    <  self.mode.height as u32);
		assert!(dst.bottom() <= self.mode.height as u32);
		
		for row in (dst.top() .. dst.bottom())
		{
			let sl = self.scanline(row as usize);
			match output_fmt
			{
			VideoFormat::X8R8G8B8 => {
				let bpp = 4;
				let left_byte  = dst.left()  as usize * bpp;
				let right_byte = dst.right() as usize * bpp;
				let seg = &mut sl[left_byte .. right_byte];
				for px in seg.chunks_mut(bpp)
				{
					px[0] = ((colour >>  0) & 0xFF) as u8;
					px[1] = ((colour >>  8) & 0xFF) as u8;
					px[2] = ((colour >> 16) & 0xFF) as u8;
					//px[3] = ((col >> 32) & 0xFF) as u8;
				}
				},
			VideoFormat::R5G6B5 => {
				let bpp = 2;
				let left_byte  = dst.left()  as usize * bpp;
				let right_byte = dst.right() as usize * bpp;
				let seg = &mut sl[left_byte .. right_byte];
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
	}
}

