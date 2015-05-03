// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video/mod.rs
///! Video (Display) management
use prelude::*;
use sync::mutex::{Mutex,LazyMutex};
use lib::sparse_vec::SparseVec;

pub use self::geom::{Pos,Dims,Rect};

module_define!{Video, [], init}

/// Pre-driver graphics support (using a bootloader provided video mode)
pub mod bootvideo;

/// Geometry types (Pos, Dims, Rect)
mod geom;


/// Handle held by framebuffer drivers
pub struct FramebufferRegistration
{
	reg_id: usize,
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

struct DisplaySurface
{
	region: Rect,
	fb: Box<Framebuffer>,
}
//struct HiddenSurface
//{
//	fb: Box<Framebuffer>,
//}

/// Sparse list of registered display devices
static S_DISPLAY_SURFACES: LazyMutex<SparseVec<DisplaySurface>> = lazymutex_init!( );
/// Boot video mode
static mut S_BOOT_MODE: Option<bootvideo::VideoMode> = None;
/// Function called when display geometry changes
static S_GEOM_UPDATE_SIGNAL: Mutex<Option<fn(new_total: Rect)>> = mutex_init!(None);

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

/// Register a function to be called when the display dimensions change
pub fn register_geom_update(fcn: fn(new_total: Rect))
{
	let mut lh = S_GEOM_UPDATE_SIGNAL.lock();
	assert!(lh.is_none(), "register_geom_update called multiple times (prev {:p}, new {:p})", lh.as_ref().unwrap(), &fcn);
	*lh = Some(fcn);
}

fn signal_geom_update(surfs: &SparseVec<DisplaySurface>)
{
	// API Requirements
	// - New surface added (with location)
	// - Old surface removed
	// - Surface relocated (as part of removal/sorting/editing)
	// > Could just have a generic "things changed" call and let the GUI/user poll request the new state
	if let Some(fcn) = *S_GEOM_UPDATE_SIGNAL.lock()
	{
		let total_area = surfs.iter().map(|x| x.region).fold(Rect::new(0,0,0,0), |a,b| a.union(&b));
		fcn( total_area );
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
			// - Remove boot video framebuffer
			S_DISPLAY_SURFACES.lock().remove(0);
			log_notice!("Alternative display driver loaded, dropping boot video");
		}
	}

	// Add new output to the global list	
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
	
	signal_geom_update(&lh);
	
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

impl ::core::ops::Drop for FramebufferRegistration
{
	fn drop(&mut self)
	{
		let mut lh = S_DISPLAY_SURFACES.lock();
		lh.remove(self.reg_id);
		signal_geom_update(&lh);
	}
}


// vim: ft=rust
