// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video.rs
// - Video (Display) management
use _common::*;
use lib::sparse_vec::SparseVec;

// DESIGN
// - Manages a set of video "heads"

module_define!{Video, [], init}

/// A handle held by users of framebuffers
pub struct FramebufferRef
{
	backing_id: usize,
}

/// Handle held by framebuffer drivers
pub struct FramebufferRegistration
{
	reg_id: usize,
}

#[derive(Copy,Clone,PartialEq,Default)]
pub struct Pos
{
	pub x: u32,
	pub y: u32,
}
#[derive(Copy,Clone,PartialEq,Default)]
pub struct Dims
{
	pub w: u32,
	pub h: u32,
}
#[derive(Copy,Clone,PartialEq,Default)]
pub struct Rect
{
	pub pos: Pos,
	pub dims: Dims,
}

/**
 * "Device"-side display surface
 *
 * A single framebuffer can be bound to multiple outputs (as a mirror).
 * Multiple separate outputs are handled with multiple framebuffers
 */
pub trait Framebuffer: 'static + Send
{
	fn as_any(&self) -> &Any;
	fn activate(&mut self);
	
	fn get_size(&self) -> Dims;
	fn set_size(&mut self, newsize: Dims) -> bool;
	
	fn blit_inner(&mut self, dst: Rect, src: Rect);
	fn blit_ext(&mut self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool;
	fn blit_buf(&mut self, dst: Rect, buf: &[u32]);
	fn fill(&mut self, dst: Rect, colour: u32);
	
	// TODO: Handle 3D units
}

pub struct DisplaySurface
{
	region: Rect,
	fb: Box<Framebuffer>,
}
//struct HiddenSurface
//{
//	fb: Box<Framebuffer>,
//}

/// Pre-driver graphics support (using a bootloader provided video mode)
pub mod bootvideo
{
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
			todo!("Framebuffer::blit_inner");
		}
		fn blit_ext(&mut self, dst: Rect, src: Rect, srf: &super::Framebuffer) -> bool {
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
			todo!("Framebuffer::fill");
		}
	}
}

/// Sparse list of registered display devices
static S_DISPLAY_SURFACES: ::sync::mutex::LazyMutex<SparseVec<DisplaySurface>> = lazymutex_init!( );
/// Boot video mode
static mut S_BOOT_MODE: Option<bootvideo::VideoMode> = None;

fn init()
{
	S_DISPLAY_SURFACES.init( || SparseVec::new() );
	
	if let Some(mode) = unsafe { S_BOOT_MODE.as_ref() }
	{
		log_notice!("Using boot video mode {:?}", mode);
		let fb = box bootvideo::Framebuffer::new(*mode) as Box<Framebuffer>;
		let dims = fb.get_size();
		S_DISPLAY_SURFACES.lock().insert( DisplaySurface {
			region: Rect::new(0,0, dims.w,dims.h),
			fb: fb
			} );
	}
	else
	{
		log_warning!("No boot video mode set");
	}
}

/// Set the boot video mode.
///
/// NOTE: Must be called before this module is initialised to have any effect
pub fn set_boot_mode(mode: bootvideo::VideoMode)
{
	unsafe {
		assert!(S_BOOT_MODE.is_none(), "Boot video mode set multiple times");
		S_BOOT_MODE = Some(mode);
	}
}

/// Add an output to the display manager
pub fn add_output(output: Box<Framebuffer>) -> FramebufferRegistration
{
	let dims = output.get_size();
	// Detect if the boot mode is still present, and clear if it is
	unsafe {
		if S_BOOT_MODE.take().is_some()
		{
			// - Create a registration for #0, aka the boot mode, and then drop it
			::core::mem::drop( FramebufferRegistration { reg_id: 0 } );
			log_notice!("Alternative display driver loaded, dropping boot video");
		}
	}
	
	let mut lh = S_DISPLAY_SURFACES.lock();
	let pos = if lh.count() == 0 {
			Pos::new(0, 0)
		} else {
			todo!("add_output: Pick a suitable location for the new surface");
		};
	let idx = lh.insert( DisplaySurface {
		region: Rect::new(pos.x,pos.y,dims.w,dims.h),
		fb: output
		} );
	
	todo!("add_output: Inform GUI of changed dimensions");
	
	log_debug!("Registering framebuffer #{}", idx);
	FramebufferRegistration {
		reg_id: idx
	}
}

/// Returns the display region that contains the given point
pub fn get_display_for_pos(pos: Pos) -> Option<Rect>
{
	for surf in S_DISPLAY_SURFACES.lock().iter_mut()
	{
		if surf.region.contains(&pos)
		{
			return Some(surf.region);
		}
	}
	None
}

/// Write part of a single scanline to the screen
///
/// Unsafe because it (eventually) will be able to cause multiple writers
pub unsafe fn write_line(pos: Pos, data: &[u32])
{
	let rect = Rect { pos: pos, dims: Dims::new(data.len() as u32, 1) };
	// 1. Locate surface
	for surf in S_DISPLAY_SURFACES.lock().iter_mut()
	{
		if let Some(sub_rect) = surf.region.intersect(&rect)
		{
			let ofs_l = sub_rect.left() - rect.left();
			let ofs_r = sub_rect.right() - rect.left();
			// 2. Blit to it
			surf.fb.blit_buf(sub_rect, &data[ofs_l as usize .. ofs_r as usize]);
		}
	}
}

impl FramebufferRef
{
	pub fn fill(&mut self, dst: Rect, colour: u32) {
		S_DISPLAY_SURFACES.lock()[self.backing_id].fb.fill(dst, colour);
	}
}

//impl ::core::ops::Drop for FramebufferRef
//{
//	fn drop(&mut self)
//	{
//	}
//}

impl ::core::ops::Drop for FramebufferRegistration
{
	fn drop(&mut self)
	{
		S_DISPLAY_SURFACES.lock().remove(self.reg_id);
	}
}

impl Pos
{
	pub fn new(x: u32, y: u32) -> Pos {
		Pos { x: x, y: y }
	}
}

impl Dims
{
	pub fn new(w: u32, h: u32) -> Dims {
		Dims { w: w, h: h }
	}

	pub fn height(&self) -> u32 { self.h }
	pub fn width(&self) -> u32 { self.w }
}

impl Rect
{
	pub fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
		Rect {
			pos: Pos { x: x, y: y },
			dims: Dims::new(w,h),
		}
	}
	pub fn new_pd(pos: Pos, dims: Dims) -> Rect {
		Rect { pos: pos, dims: dims }
	}
	
	pub fn within(&self, w: u32, h: u32) -> bool {
		self.x() < w && self.y() < h
		&& self.w() <= w && self.h() <= h
		&& self.x() + self.w() <= w && self.y() + self.h() <= h
	}
	
	pub fn pos(&self) -> Pos { self.pos }
	pub fn dims(&self) -> Dims { self.dims }
	
	pub fn x(&self) -> u32 { self.pos.x }
	pub fn y(&self) -> u32 { self.pos.y }
	pub fn w(&self) -> u32 { self.dims.w }
	pub fn h(&self) -> u32 { self.dims.h }
	
	pub fn top(&self) -> u32 { self.y() }
	pub fn left(&self) -> u32 { self.x() }
	pub fn right(&self) -> u32 { self.x() + self.w() }
	pub fn bottom(&self) -> u32 { self.y() + self.h() }
	
	pub fn tl(&self) -> Pos { self.pos }
	pub fn br(&self) -> Pos { Pos::new( self.x() + self.w(), self.y() + self.h() ) }
	pub fn br_inner(&self) -> Pos { Pos::new( self.x() + self.w() - 1, self.y() + self.h() - 1 ) }
	
	pub fn contains(&self, pt: &Pos) -> bool {
		//log_trace!("Rect::contains - self={:?}, pt={:?}", self, pt);
		(self.left() <= pt.x && pt.x < self.right()) && (self.top() <= pt.y && pt.y < self.bottom())
	}
	pub fn contains_rect(&self, r: &Rect) -> bool {
		//log_trace!("Rect::contains - self={:?}, pt={:?}", self, pt);
		if ! self.contains( &r.tl() ) {
			false
		}
		else if r.w() == 0 || r.h() == 0 {
			true
		}
		else if self.contains( &r.br_inner() ) {
			true
		}
		else {
			false
		}
	}
	
	pub fn intersect(&self, other: &Rect) -> Option<Rect> {
		// Intersection:
		//  MAX(X1) MAX(Y1)  MIN(X2) MIN(Y2)
		let max_x1 = ::core::cmp::max( self.left(), other.left() );
		let max_y1 = ::core::cmp::max( self.top() , other.top() );
		let min_x2 = ::core::cmp::min( self.right() , other.right() );
		let min_y2 = ::core::cmp::min( self.bottom(), other.bottom() );
		
		//log_trace!("Rect::intersect({} with {}) = ({},{}) ({},{})", self, other, max_x1, max_y1, min_x2, min_y2);
		
		if max_x1 < min_x2 && max_y1 < min_y2 {
			Some( Rect {
				pos: Pos { x: max_x1, y: max_y1 },
				dims: Dims::new((min_x2 - max_x1), (min_y2 - max_y1))
				} )
		}
		else {
			None
		}
	}
	
	/// Iterate over intersections of two slices of `Rect`
	pub fn list_intersect<'a>(list1: &'a [Rect], list2: &'a [Rect]) -> RectListIntersect<'a> {
		RectListIntersect {
			list1: list1,
			list2: list2,
			idx1: 0,
			idx2: 0,
		}
	}
}
pub struct RectListIntersect<'a>
{
	list1: &'a [Rect],
	list2: &'a [Rect],
	idx1: usize,
	idx2: usize,
}
impl<'a> Iterator for RectListIntersect<'a>
{
	type Item = Rect;
	fn next(&mut self) -> Option<Rect>
	{
		// Iterate list1, iterate list2
		while self.idx1 < self.list1.len()
		{
			if self.idx2 == self.list2.len() {
				self.idx1 += 1;
				self.idx2 = self.idx1;
				if self.idx2 >= self.list2.len() {
					return None;
				}
			}
			else {
				let rv = self.list1[self.idx1].intersect( &self.list2[self.idx2] );
				self.idx2 += 1;
				if rv.is_some() {
					return rv;
				}
			}
		}
		None
	}
}

impl_fmt! {
	Debug(self, f) for Pos { write!(f, "({},{})", self.x, self.y) }
	Debug(self, f) for Dims { write!(f, "{}x{}", self.w, self.h) }
	Debug(self, f) for Rect { write!(f, "({},{} + {}x{})", self.x(), self.y(), self.w(), self.h()) }
	Display(self, f) for Rect { write!(f, "({},{} + {}x{})", self.x(), self.y(), self.w(), self.h()) }
}

// vim: ft=rust
