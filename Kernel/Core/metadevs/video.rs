// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video.rs
// - Video (Display) output arbitration
use _common::*;

// DESIGN
// - Manages a set of video "heads"

module_define!(Video, [], init)

/**
 * "Client"-side display surface handle
 * Represents a (possibly) visible framebuffer
 */
pub struct Display
{
	backing_id: uint,
}
pub struct FramebufferRegistration
{
	reg_id: uint,
}

pub struct Rect
{
	x: u16,
	y: u16,
	w: u16,
	h: u16,
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
	fn set_size(&self, newsize: Rect) -> bool;
	
	fn blit_inner(&self, dst: Rect, src: Rect);
	fn blit_ext(&self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool;
	fn blit_buf(&self, dst: Rect, buf: &[u32]);
	fn fill(&self, dst: Rect, colour: u32);
}

//static s_display_surfaces: Mutex<Vec<Box<Framebuffer+'static>>> = mutex_init!( empty_vec!() );

fn init()
{
	// TODO: What init would the display processor need?
}

pub fn add_output(output: Box<Framebuffer+Send>) -> FramebufferRegistration
{
	panic!("TODO: video::add_output");
}

impl Rect
{
	pub fn new(x: u16, y: u16, w: u16, h: u16) -> Rect {
		Rect {
			x: x, y: y,
			w: w, h: h,
		}
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
