// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video.rs
// - Video (Display) output arbitration
use _common::*;

// DESIGN
// - Manages a set of video "heads"

module_define!{Video, [], init}

// /**
// * "Client"-side display surface handle
// * Represents a (possibly) visible framebuffer
// */
//pub struct Display
//{
//	backing_id: uint,
//}

/// Handle held by framebuffer drivers
pub struct FramebufferRegistration
{
	reg_id: usize,
}

#[derive(Copy,Clone,PartialEq,Debug)]
pub struct Rect
{
	pub x: u16,
	pub y: u16,
	pub w: u16,
	pub h: u16,
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
	
	fn get_size(&self) -> Rect;
	fn set_size(&mut self, newsize: Rect) -> bool;
	
	fn blit_inner(&mut self, dst: Rect, src: Rect);
	fn blit_ext(&mut self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool;
	fn blit_buf(&mut self, dst: Rect, buf: &[u32]);
	fn fill(&mut self, dst: Rect, colour: u32);
	
	// TODO: Handle 3D units
}

/// Sparse list of registered display devices
static S_DISPLAY_SURFACES: ::sync::mutex::LazyMutex<Vec<Option<Box<Framebuffer>>>> = lazymutex_init!( );

fn init()
{
	// TODO: What init would the display processor need?
	S_DISPLAY_SURFACES.init( || Vec::new() );
}

//impl ::core::ops::Drop for Display
//{
//	fn drop(&mut self)
//	{
//	}
//}

pub fn add_output(output: Box<Framebuffer>) -> FramebufferRegistration
{
	let mut lh = S_DISPLAY_SURFACES.lock();
	lh.push(Some(output));
	log_debug!("Registering framebuffer #{}", lh.len());
	FramebufferRegistration {
		reg_id: lh.len()
	}
}

impl ::core::ops::Drop for FramebufferRegistration
{
	fn drop(&mut self)
	{
		S_DISPLAY_SURFACES.lock()[self.reg_id] = None;
	}
}

impl Rect
{
	pub fn new(x: u16, y: u16, w: u16, h: u16) -> Rect {
		Rect {
			x: x, y: y,
			w: w, h: h,
		}
	}
	
	pub fn within(&self, w: u16, h: u16) -> bool {
		self.x < w && self.y < h
		&& self.w <= w && self.h <= h
		&& self.x + self.w <= w && self.y + self.h <= h
	}
}

impl ::core::fmt::Display for Rect
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "({},{} + {}x{})", self.x, self.y, self.w, self.h)
	}
}

// vim: ft=rust
