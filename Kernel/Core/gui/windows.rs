// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/windows.rs
// - GUI Window management
use _common::*;
use super::{Dims,Pos,Rect,Colour};
use sync::mutex::{LazyMutex,Mutex};
use sync::rwlock::RwLock;
use lib::mem::Arc;

use lib::sparse_vec::SparseVec;

/// Window groups combine windows into "sessions", that can be switched with magic key combinations
struct WindowGroup
{
	/// Window group name, may be shown to the user if requested
	name: String,
	/// Canonical list of windows (sparse, for reallocation of IDs)
	windows: SparseVec<Window>,
	/// Render order (indexes into `windows`, and visibilities)
	render_order: Vec< (usize, Vec<Rect>) >,
}

#[derive(Default)]
struct WinBuf
{
	/// Window dimensions
	dims: Dims,
	/// Window backing buffer
	data: Vec<u32>,
}

/// A single window, an arbitarily movable on-screen region
#[derive(Default)]
struct Window
{
	/// Position relative to the top-left of the display
	position: Pos,
	
	/// Actual window data
	/// 
	/// Write lock is for structure manipulations, slicing is sharable
	buf: RwLock<WinBuf>,
	
	/// Window title (queried by the decorator)
	title: String,
	
	/// List of invalidated regions within the window
	dirty_rects: Vec<Rect>,
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
		for &(winidx,ref vis) in &self.render_order
		{
			let vis = &vis[..];
			let win = &mut self.windows[winidx];
			// 1. Is the window dirty, or are we doing a full redraw
			if full || win.is_dirty()
			{
				static FULL_RECT: [Rect; 1] = [Rect(Pos(0,0),Dims(!0,!0))];
				
				// 2. Obtain the visible sections of this window that have changed
				let dirty = if full { &FULL_RECT[..] } else { &win.dirty_rects[..] };
				for rgn in Rect::list_intersect(vis, dirty)
				{
					// Blit data from the window to the screen
					win.buf.read().blit(win.position, rgn);
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
		// - Get handle to this window group (ok to lock it)
		let wgh_rc = S_WINDOW_GROUPS.lock()[self.0].clone();
		
		let idx = wgh_rc.lock().windows.insert(Window::default());
		WindowHandle { grp: self.0, win: idx }
	}
}
impl ::core::ops::Drop for WindowGroupHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

impl WinBuf
{
	fn resize(&mut self, newsize: Dims)
	{
		unimplemented!();
	}
	
	fn scanline_rgn(&self, line: usize, ofs: usize, len: usize) -> &[u32]
	{
		assert!(ofs < self.dims.width() as usize);
		assert!(line < self.dims.1 as usize, "Requested scanline is out of range");
		
		let pitch_32 = self.dims.width() as usize;
		let len = ::core::cmp::max(len, pitch_32 - ofs);
		
		let l_ofs = line * pitch_32;
		&self.data[ l_ofs + ofs .. l_ofs + ofs + len ] 
	}

	fn blit(&self, winpos: Pos, rgn: Rect)
	{
		// TODO: Call a block blit instead?
		for row in rgn.top() .. rgn.bottom()
		{
			self.blit_scanline(
				winpos,
				row as usize,
				rgn.left() as usize,
				rgn.dim().width() as usize
				);
		}
	}
	
	fn blit_scanline(&self, winpos: Pos, line: usize, ofs: usize, len: usize)
	{
		// TODO: Assert that current thread is from/controlled-by the compositor
		unsafe {
			let pos = ::metadevs::video::Pos::new(
				winpos.x() as u16 + ofs as u16,
				winpos.y() as u16 + line as u16
				);
			::metadevs::video::write_line(pos, self.scanline_rgn(line, ofs, len));
		}
	}
	
	fn scanline_rgn_mut(&mut self, line: usize, ofs: usize, len: usize) -> &mut [u32]
	{
		assert!(ofs < self.dims.width() as usize);
		assert!(line < self.dims.1 as usize, "Requested scanline is out of range");
		
		let pitch_32 = self.dims.width() as usize;
		let len = ::core::cmp::max(len, pitch_32 - ofs);
		
		let l_ofs = line * pitch_32;
		&mut self.data[ l_ofs + ofs .. l_ofs + ofs + len ] 
	}
	
	fn fill_scanline(&mut self, line: usize, ofs: usize, len: usize, value: Colour)
	{
		if line >= self.dims.height() as usize || ofs >= self.dims.width() as usize {
			return ;
		}
		let rgn = self.scanline_rgn_mut(line, ofs, len);
		for v in rgn.iter_mut()
		{
			*v = value.as_argb32();
		}
	}
	fn set_scanline(&mut self, line: usize, ofs: usize, len: usize, data: &[u32])
	{
		if line >= self.dims.height() as usize || ofs >= self.dims.width() as usize {
			return ;
		}
		let rgn = self.scanline_rgn_mut(line, ofs, len);
		
		for (d,s) in rgn.iter_mut().zip( data.iter() )
		{
			*d = *s;
		}
	}
}

impl Window
{
	fn is_dirty(&self) -> bool { self.dirty_rects.len() > 0 }
	
	pub fn fill_rect(&self, area: Rect, colour: Colour)
	{
		let mut buf_h = self.buf.write();
		for row in area.top() .. area.bottom()
		{
			buf_h.fill_scanline(
				row as usize,
				area.left() as usize,
				area.dim().width() as usize,
				colour
				);
		}
	}
	
	pub fn pset(&self, pos: Pos, colour: Colour)
	{
		self.buf.write().fill_scanline(pos.1 as usize, pos.0 as usize, 1, colour);
	}
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
	}
	
	/// Fill an area of the window with a specific colour
	pub fn fill_rect(&mut self, area: Rect, colour: Colour)
	{
		log_trace!("(area={:?},colour={:?})", area, colour);
		// TODO: Avoid having to lock the global list here
		let wgh = S_WINDOW_GROUPS.lock()[self.grp].clone();
		// TODO: DEFINITELY avoid locking the WG
		wgh.lock().windows[self.win].fill_rect(area, colour);
	}
	
	/// Set single pixel (VERY inefficient, don't use unless you need to)
	pub fn pset(&mut self, pos: Pos, colour: Colour)
	{
		//log_trace!("(pos={:?},colour={:?})", pos, colour);
		// TODO: Avoid having to lock the global list here
		let wgh = S_WINDOW_GROUPS.lock()[self.grp].clone();
		// TODO: DEFINITELY avoid locking the WG
		wgh.lock().windows[self.win].pset(pos, colour);
	}
}

impl ::core::ops::Drop for WindowHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

