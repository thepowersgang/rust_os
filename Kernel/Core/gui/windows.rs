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
use core::atomic;

use lib::sparse_vec::SparseVec;

/// Window groups combine windows into "sessions", that can be switched with magic key combinations
struct WindowGroup
{
	/// Window group name, may be shown to the user if requested
	name: String,
	/// Canonical list of windows (sparse, for reallocation of IDs)
	///
	/// Contains both the window position and shared ownership of the window data.
	/// Position is here because the window itself doesn't need control (or knowledge) of its position
	windows: SparseVec< (Pos, Arc<Window>) >,
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
//#[derive(Default)]
struct Window
{
	/// Actual window data
	/// 
	/// Write lock is for structure manipulations, slicing is sharable
	buf: RwLock<WinBuf>,
	
	/// Window title (queried by the decorator)
	title: String,
	
	/// List of invalidated regions within the window
	dirty_rects: Mutex<Vec<Rect>>,
	is_dirty: atomic::AtomicBool,
	
	
	/// If true, the window is maximised, and should be resized with the screen
	is_maximised: bool,
}
impl ::core::default::Default for Window {
	fn default() -> Window {
		use core::default::Default;
		Window {
			buf: Default::default(),
			title: Default::default(),
			dirty_rects: Default::default(),
			is_dirty: atomic::ATOMIC_BOOL_INIT,
			is_maximised: false,
		}
	}
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

pub fn update_dims()
{
	// Iterate all "maximised" windows
	//{
	//	let dims = ::metadevs::video::get_dims_at(win.position);
	//}
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
		
		log_debug!("render_thread: Rendering WG {}", grp_idx);
		grp_ref.lock().redraw(false);
	}
}

impl WindowGroup
{
	/// Re-draw this window group
	fn redraw(&mut self, full: bool)
	{
		log_trace!("WindowGroup::redraw: render_order={:?}", self.render_order);
		for &(winidx,ref vis) in &self.render_order
		{
			let vis = &vis[..];
			let (ref pos, ref win) = self.windows[winidx];
			// 1. Is the window dirty, or are we doing a full redraw
			if win.take_is_dirty() || full
			{
				static FULL_RECT: [Rect; 1] = [Rect { pos: Pos {x:0,y:0},dims: Dims{w:!0,h:!0}}];
				
				// 2. Obtain the visible sections of this window that have changed
				// - Switch the dirty rect out with an empty Vec
				let dirty_vec = ::core::mem::replace(&mut *win.dirty_rects.lock(), Vec::new());
				// - Get a slice of it (OR, if doing a full re-render, get a wildcard region)
				let dirty = if full { &FULL_RECT[..] } else { &dirty_vec[..] };
				log_trace!("WindowGroup::redraw: dirty={:?}, vis={:?}", dirty, vis);
				// - Iterate all visible dirty regions and re-draw
				for rgn in Rect::list_intersect(vis, dirty)
				{
					// Blit data from the window to the screen
					win.buf.read().blit(*pos, rgn);
				}
			}
		}
	}

	/// Obtains the render position of the specified window
	fn get_render_idx(&self, winidx: usize) -> Option<usize> {
		self.render_order.iter().position( |&(idx,_)| idx == winidx )
	}
		
	/// Recalculate the cached visibility regions caused by 'changed_idx' updating
	fn recalc_vis(&mut self, changed_idx: usize)
	{
		// Changed visibility only affects this window and lower windows.
		if let Some(idx) = self.get_render_idx(changed_idx)
		{
			self.recalc_vis_int(idx);
		}
	}
	/// Recalculate visibility information for all windows below (and including) the specified render position
	fn recalc_vis_int(&mut self, vis_idx: usize)
	{
		// For each window this one and below
		for i in (0 .. vis_idx+1).rev()
		{
			// Iterate all higher windows and obtain visible rects
			self.render_order[i].1 = self.recalc_vis_for(i);
		}
	}
	/// Recalculate the visibility vector for a specific window in the render order
	fn recalc_vis_for(&mut self, vis_idx: usize) -> Vec<Rect>
	{
		let win_idx = self.render_order[vis_idx].0;
		let dims = self.windows[win_idx].1.buf.read().dims;
		let mut vis = vec![ Rect { pos: Pos::new(0,0), dims: dims } ];
		for &(win,_) in &self.render_order[ vis_idx+1 .. ]
		{
			todo!("WindowGroup::recalc_vis_int(vis_idx={})", vis_idx);
		}
		vis
	}
	
	fn show_window(&mut self, idx: usize) {
		if self.get_render_idx(idx).is_some() {
			return ;
		}
		let rect = Rect { pos: self.windows[idx].0, dims: self.windows[idx].1.buf.read().dims };
		self.render_order.push( (idx, vec![rect]) );
		let vis_idx = self.render_order.len() - 1;
		self.recalc_vis_int(vis_idx);
	}
	fn hide_window(&mut self, idx: usize) {
		if let Some(pos) = self.get_render_idx(idx)
		{
			todo!("WindowGroup::hide_window({}) - pos={}", idx, pos);
		}
	}
	
	fn maximise_window(&mut self, idx: usize) {
		{
			let &mut(ref mut pos, ref win_rc) = &mut self.windows[idx];
			let rect = match ::metadevs::video::get_display_for_pos(*pos)
				{
				Some(x) => x,
				None => todo!("Handle window being off-screen"),
				};
			// - Move window to new position
			*pos = rect.pos();
			// - Resize window
			win_rc.resize(rect.dims());
		}
		// Recalculate visible regions
		self.recalc_vis(idx);
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
		
		let idx = wgh_rc.lock().windows.insert( (Pos::new(0,0), Arc::new(Window::default())) );
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
		log_trace!("WinBuf::resize({:?})", newsize);
		let px_count = newsize.width() as usize * newsize.height() as usize;
		log_trace!("- px_count = {}", px_count);
		self.dims = newsize;
		self.data.resize(px_count, 0);
	}
	
	fn scanline_rgn(&self, line: usize, ofs: usize, len: usize) -> &[u32]
	{
		assert!(ofs < self.dims.width() as usize);
		assert!(line < self.dims.h as usize, "Requested scanline is out of range");
		
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
				rgn.dims().width() as usize
				);
		}
	}
	
	fn blit_scanline(&self, winpos: Pos, line: usize, ofs: usize, len: usize)
	{
		// TODO: Assert that current thread is from/controlled-by the compositor
		unsafe {
			let pos = Pos::new(
				winpos.x + ofs as u32,
				winpos.y + line as u32
				);
			::metadevs::video::write_line(pos, self.scanline_rgn(line, ofs, len));
		}
	}
	
	fn scanline_rgn_mut(&mut self, line: usize, ofs: usize, len: usize) -> &mut [u32]
	{
		assert!(ofs < self.dims.width() as usize);
		assert!(line < self.dims.h as usize, "Requested scanline is out of range");
		
		let pitch_32 = self.dims.width() as usize;
		let len = ::core::cmp::max(len, pitch_32 - ofs);
		
		let l_ofs = line * pitch_32;
		//log_debug!("scanline_rgn_mut: self.data = {:p}", &self.data[0]);
		&mut self.data[ l_ofs + ofs .. l_ofs + ofs + len ] 
	}
	
	fn fill_scanline(&mut self, line: usize, ofs: usize, len: usize, value: Colour)
	{
		if line >= self.dims.height() as usize || ofs >= self.dims.width() as usize {
			return ;
		}
		let rgn = self.scanline_rgn_mut(line, ofs, len);
		//log_debug!("fill_scanline: rgn = {:p}", &rgn[0]);
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
	fn take_is_dirty(&self) -> bool { self.is_dirty.swap(false, atomic::Ordering::Relaxed) }
	fn mark_dirty(&self) { self.is_dirty.store(true, atomic::Ordering::Relaxed); }
	
	pub fn resize(&self, dim: Dims) {
		self.buf.write().resize(dim);
	}
	
	fn add_dirty(&self, area: Rect)
	{
		todo!("Window::add_dirty({:?})", area);
	}
	
	pub fn fill_rect(&self, area: Rect, colour: Colour)
	{
		let mut buf_h = self.buf.write();
		for row in area.top() .. area.bottom()
		{
			buf_h.fill_scanline(
				row as usize,
				area.left() as usize,
				area.dims().w as usize,
				colour
				);
		}
		self.add_dirty(area);
	}
	
	pub fn pset(&self, pos: Pos, colour: Colour)
	{
		self.buf.write().fill_scanline(pos.y as usize, pos.x as usize, 1, colour);
		self.add_dirty( Rect::new_pd(pos, Dims::new(1,1)) );
	}
}

impl WindowHandle
{
	fn get_wg(&self) -> Arc<Mutex<WindowGroup>> {
		S_WINDOW_GROUPS.lock()[self.grp].clone()
	}
	fn get_win(&self) -> Arc<Window> {
		let wg = self.get_wg();
		let win_arc: &Arc<Window> = &wg.lock().windows[self.win].1;
		win_arc.clone()
	}
	fn get_win_w_pos(&self) -> (Pos, Arc<Window>) {
		let wg = self.get_wg();
		let wgl = wg.lock();
		let win = &wgl.windows[self.win];
		(win.0, win.1.clone())
	}
	
	/// Poke the window group and tell it that it needs to recalculate visibilities
	fn trigger_recalc_vis(&self) {
		let wg = self.get_wg();
		wg.lock().recalc_vis(self.win);
	}
	
	/// Redraw the window (mark for re-blitting)
	pub fn redraw(&mut self)
	{
		// if shown, mark self as requiring reblit and poke group
		if self.grp != S_CURRENT_GROUP.load(::core::atomic::Ordering::Relaxed) {
			return ;
		}
		
		self.get_win().mark_dirty();
		S_RENDER_REQUEST.post();
	}
	
	/// Resize the window
	pub fn resize(&mut self, dim: Dims) {
		self.get_win().resize(dim);
		self.trigger_recalc_vis();
	}
	/// Maximise this window (fill all space on the current monitor)
	pub fn maximise(&mut self) {
		let wg = self.get_wg();
		wg.lock().maximise_window( self.win );
		// TODO: Set maximised flag so that the window gets updated on screen changes
		// No need to call trigger_recalc_vis, maximise_window does that
	}
	/// Show the window
	pub fn show(&mut self) {
		let wg = self.get_wg();
		wg.lock().show_window( self.win );
	}
	/// Hide the window
	pub fn hide(&mut self) {
		let wg = self.get_wg();
		wg.lock().hide_window( self.win );
	}
	
	/// Fill an area of the window with a specific colour
	pub fn fill_rect(&mut self, area: Rect, colour: Colour)
	{
		log_trace!("(area={:?},colour={:?})", area, colour);
		self.get_win().fill_rect(area, colour);
	}
	
	/// Set single pixel (VERY inefficient, don't use unless you need to)
	pub fn pset(&mut self, pos: Pos, colour: Colour)
	{
		//log_trace!("(pos={:?},colour={:?})", pos, colour);
		self.get_win().pset(pos, colour);
	}
}

impl ::core::ops::Drop for WindowHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

