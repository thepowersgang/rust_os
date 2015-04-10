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

#[derive(Copy,Clone,PartialEq,Debug)]
pub struct Pos
{
	pub x: u16,
	pub y: u16,
}
#[derive(Copy,Clone,PartialEq,Debug)]
pub struct Dims
{
	pub w: u16,
	pub h: u16,
}
#[derive(Copy,Clone,PartialEq,Debug)]
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

pub mod bootvideo
{
	#[derive(Copy,Clone,Debug)]
	pub enum VideoFormat
	{
		X8R8G8B8,
		B8G8R8X8,
		R8G8B8,
		B8G8R8,
		R5G6B5,
	}

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
		
	}
	else
	{
		log_warning!("No boot video mode set");
	}
}

pub fn set_boot_mode(mode: bootvideo::VideoMode)
{
	unsafe {
		assert!(S_BOOT_MODE.is_none(), "Boot video mode set multiple times");
		S_BOOT_MODE = Some(mode);
	}
}

pub fn add_output(output: Box<Framebuffer>) -> FramebufferRegistration
{
	let dims = output.get_size();
	let mut lh = S_DISPLAY_SURFACES.lock();
	let idx = lh.insert( DisplaySurface {
		region: Rect::new(0,0,dims.w,dims.h),
		fb: output
		} );
	log_debug!("Registering framebuffer #{}", idx);
	FramebufferRegistration {
		reg_id: idx
	}
}

/// Write part of a single scanline to the screen
pub fn write_line(pos: Pos, data: &[u32])
{
	let rect = Rect { pos: pos, dims: Dims::new(data.len() as u16, 1) };
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
	pub fn new(x: u16, y: u16) -> Pos {
		Pos { x: x, y: y }
	}
}

impl Dims
{
	pub fn new(w: u16, h: u16) -> Dims {
		Dims { w: w, h: h }
	}
}

impl Rect
{
	pub fn new(x: u16, y: u16, w: u16, h: u16) -> Rect {
		Rect {
			pos: Pos { x: x, y: y },
			dims: Dims::new(w,h),
		}
	}
	
	pub fn within(&self, w: u16, h: u16) -> bool {
		self.x() < w && self.y() < h
		&& self.w() <= w && self.h() <= h
		&& self.x() + self.w() <= w && self.y() + self.h() <= h
	}
	
	pub fn x(&self) -> u16 { self.pos.x }
	pub fn y(&self) -> u16 { self.pos.y }
	pub fn w(&self) -> u16 { self.dims.w }
	pub fn h(&self) -> u16 { self.dims.h }
	
	pub fn top(&self) -> u16 { self.y() }
	pub fn left(&self) -> u16 { self.x() }
	pub fn right(&self) -> u16 { self.x() + self.w() }
	pub fn bottom(&self) -> u16 { self.y() + self.h() }
	
	pub fn intersect(&self, other: &Rect) -> Option<Rect> {
		// Intersection:
		//  MAX(X1) MAX(Y1)  MIN(X2) MIN(Y2)
		let max_x1 = ::core::cmp::max( self.left(), other.left() );
		let max_y1 = ::core::cmp::max( self.top() , other.top() );
		let min_x2 = ::core::cmp::min( self.right() , other.right() );
		let min_y2 = ::core::cmp::min( self.bottom(), other.bottom() );
		
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
}

impl_fmt! {
	Display(self, f) for Rect { write!(f, "({},{} + {}x{})", self.x(), self.y(), self.w(), self.h()) }
}

// vim: ft=rust
