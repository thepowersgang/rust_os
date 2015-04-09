// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/windows.rs
// - GUI Window management
use _common::*;
use super::{Dims,Pos,Rect,Colour};
use sync::mutex::{LazyMutex,Mutex};
use lib::mem::Arc;

use lib::sparse_vec::SparseVec;

/// Window groups combine windows into "sessions", that can be switched with magic key combinations
struct WindowGroup
{
	/// Window group name, may be shown to the user if requested
	name: String,
	/// Canonical list of windows (sparse, for reallocation of IDs)
	windows: SparseVec<Window>,
	/// Render order (indexes into `windows`)
	render_order: Vec<usize>,
}

struct WinBuf
{
	/// Window dimensions
	dims: Dims,
	/// Window backing buffer
	data: Vec<u32>,
}

/// A single window, an arbitarily movable on-screen region
struct Window
{
	/// Position relative to the top-left of the display
	position: Pos,
	
	buf: WinBuf,
	
	/// Window title (queried by the decorator)
	title: String,
	
	/// List of invalidated regions within the window
	dirty_rects: Vec<Rect>,
	
	/// List of regions visible (i.e. those exposed to the screen)
	///
	/// This is updated whenever a window above is moved
	visible_rects: Vec<Rect>,
}

pub struct WindowGroupHandle(usize);
pub struct WindowHandle
{
	grp: usize,
	win: usize,
}

// - 13 sessions, #0 is fixed to be the kernel's log 1-12 are bound to F1-F12
const C_MAX_SESSIONS: usize = 13;
// TODO: When associated statics are implemented, replace this with a non-lazy mutex.
static S_WINDOW_GROUPS: LazyMutex<SparseVec< Arc<Mutex<WindowGroup>> >> = lazymutex_init!();
static S_CURRENT_GROUP: ::core::atomic::AtomicUsize = ::core::atomic::ATOMIC_USIZE_INIT;
static S_RENDER_REQUEST: ::sync::EventChannel = ::sync::EVENTCHANNEL_INIT;
static S_RENDER_THREAD: LazyMutex<::threads::ThreadHandle> = lazymutex_init!();

pub fn init()
{
	S_WINDOW_GROUPS.init( || SparseVec::new() );
	
	// Create render thread
	S_RENDER_THREAD.init( || ::threads::ThreadHandle::new("GUI Compositor", render_thread) );
}

// Thread that controls compiositing windows to the screen
fn render_thread()
{
	log_debug!("GUI Render Thread started");
	loop
	{
		// Wait for a signal to start a render
		S_RENDER_REQUEST.sleep();
		
		// render the active window group
		let grp_idx = S_CURRENT_GROUP.load( ::core::atomic::Ordering::Relaxed );
		let grp_ref = S_WINDOW_GROUPS.lock()[grp_idx].clone();
		
		grp_ref.lock().redraw(false);
	}
}

impl WindowGroup
{
	fn redraw(&mut self, full: bool)
	{
		for &winidx in &self.render_order
		{
			let win = &mut self.windows[winidx];
			// 1. Is the window dirty, or are we doing a full redraw
			if full || win.is_dirty()
			{
				static FULL_RECT: [Rect; 1] = [Rect(Pos(0,0),Dims(!0,!0))];
				
				// 2. Obtain the visible sections of this window that have changed
				let vis = &win.visible_rects[..];
				let dirty = if full { &FULL_RECT[..] } else { &win.dirty_rects[..] };
				for rgn in Rect::list_intersect(vis, dirty)
				{
					// Blit data from the window to the screen
					log_debug!("TODO: Blit {:?}", rgn);
				}
			}
			win.dirty_rects.clear();
		}
	}
}

impl WindowGroupHandle
{
	pub fn alloc(name: &str) -> WindowGroupHandle {
		let new_group = Arc::new( Mutex::new( WindowGroup {
			name: From::from(name),
			windows: SparseVec::new(),
			render_order: Vec::new(),
			} ) );
		// Locate unused slot
		let idx = {
			let mut grps = S_WINDOW_GROUPS.lock();
			if grps.count() == C_MAX_SESSIONS
			{
				panic!("TODO: Handle exceeding session limit");
			}
			else
			{
				grps.insert(new_group)
			}
			};
		WindowGroupHandle(idx)
	}
	
	pub fn create_window(&mut self) -> WindowHandle {
		// Allocate a new window from the list
		panic!("TODO: WindowGroupHandle::create_window()");
	}
}
impl ::core::ops::Drop for WindowGroupHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

impl Window
{
	fn is_dirty(&self) -> bool { self.dirty_rects.len() > 0 }
}

impl WindowHandle
{
	/// Redraw the window (mark for re-blitting)
	pub fn redraw(&mut self)
	{
		// if shown, mark self as requiring reblit and poke group
		if self.grp != S_CURRENT_GROUP.load(::core::atomic::Ordering::Relaxed) {
			return ;
		}
		
		S_RENDER_REQUEST.post();
		panic!("TODO: Mark window as ready for reblit");
	}
	
	/// Fill an area of the window with a specific colour
	pub fn fill_rect(&mut self, area: Rect, colour: Colour)
	{
		log_trace!("(area={:?},colour={:?})", area, colour);
		unimplemented!();
	}
	
	/// Set single pixel (VERY inefficient, don't use unless you need to)
	pub fn pset(&mut self, pos: Pos, colour: Colour)
	{
		log_trace!("(pos={:?},colour={:?})", pos, colour);
		//self.scanline(pos.row())[pos.col()] = colour;
	}
}

impl ::core::ops::Drop for WindowHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

