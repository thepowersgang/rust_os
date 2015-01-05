// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video.rs
// - Video (Display) output arbitration
use _common::*;

// DESIGN
// - Manages a set of video "heads"

module_define!{Video, [], init}

///**
// * "Client"-side display surface handle
// * Represents a (possibly) visible framebuffer
// */
//pub struct Display
//{
//	backing_id: uint,
//}
pub struct FramebufferRegistration
{
	reg_id: uint,
}

#[derive(Copy,PartialEq)]
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
pub trait Framebuffer: Any
{
	fn get_size(&self) -> Rect;
	fn set_size(&mut self, newsize: Rect) -> bool;
	
	fn blit_inner(&mut self, dst: Rect, src: Rect);
	fn blit_ext(&mut self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool;
	fn blit_buf(&mut self, dst: Rect, buf: &[u32]);
	fn fill(&mut self, dst: Rect, colour: u32);
}

// Workaround for AnyRefExt not being implemented on &Framebuffer by default
any_for_trait!{ Framebuffer }

#[allow(non_upper_case_globals)]
static s_display_surfaces: ::sync::mutex::LazyMutex<Vec<Option<Box<Framebuffer+Send>>>> = lazymutex_init!( );

fn init()
{
	// TODO: What init would the display processor need?
}

//impl ::core::ops::Drop for Display
//{
//	fn drop(&mut self)
//	{
//	}
//}

pub fn add_output(output: Box<Framebuffer+Send>) -> FramebufferRegistration
{
	let mut lh = s_display_surfaces.lock( | | Vec::new() );
	lh.push(Some(output));
	FramebufferRegistration {
		reg_id: lh.len()
	}
}

impl ::core::ops::Drop for FramebufferRegistration
{
	fn drop(&mut self)
	{
		s_display_surfaces.lock( | | Vec::new() )[self.reg_id] = None;
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

impl ::core::fmt::Show for Rect
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "")
	}
}

// vim: ft=rust
