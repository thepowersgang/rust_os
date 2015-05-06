// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/windows/mod.rs
// - GUI Window management
use prelude::*;
use super::{Dims,Pos,Rect,Colour};
use sync::mutex::{LazyMutex,Mutex};
use sync::rwlock::RwLock;
use lib::mem::Arc;
use lib::LazyStatic;
use core::atomic;

use lib::sparse_vec::SparseVec;

pub use self::winbuf::WinBuf;

/// Handle to the backing buffer of a window
pub type BufHandle = Arc<WinBuf>;

mod winbuf;

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


/// A single window, an arbitarily movable on-screen region
//#[derive(Default)]
struct Window
{
	/// Window name (set once, never changes after)
	name: String,
	
	/// Actual window data
	/// 
	/// Write lock is for structure manipulations, slicing is sharable.
	/// Arc allows the "user" to hold a copy of the framebuffer
	buf: RwLock<Arc<WinBuf>>,
	
	/// Window title (queried by the decorator)
	title: String,
	
	/// List of invalidated regions within the window
	dirty_rects: Mutex<Vec<Rect>>,
	is_dirty: atomic::AtomicBool,
	
	/// Flags on the window
	flags: Mutex<WindowFlags>,
}

#[derive(Default)]
struct WindowFlags
{
	/// If true, the window is maximised, and should be resized with the screen
	maximised: bool,
	
	///// If true, the group's decorator should skip this window
	//undecorated: bool,
}

/// Handle on a window group (owning, when dropped the group is destroyed)
pub struct WindowGroupHandle(usize);

/// Window handle (when dropped, the window is destroyed)
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
static S_EVENT_QUEUE: LazyStatic<::lib::ring_buffer::AtomicRingBuf<super::input::Event>> = lazystatic_init!();
static S_RENDER_THREAD: LazyMutex<::threads::ThreadHandle> = lazymutex_init!();

pub fn init()
{
	S_WINDOW_GROUPS.init( || SparseVec::new() );
	
	// Create render thread
	unsafe { S_EVENT_QUEUE.prep(|| ::lib::ring_buffer::AtomicRingBuf::new(32)); }
	S_RENDER_THREAD.init( || ::threads::ThreadHandle::new("GUI Compositor", render_thread) );
}


/// Update window dimensions and positions after the display organsisation changes
pub fn update_dims()
{
	// Iterate all windows
	for grp in S_WINDOW_GROUPS.lock().iter()
	{
		let mut lh = grp.lock();
		for &mut (ref mut pos, ref win) in lh.windows.iter_mut()
		{
			// Locate screen for the upper-left corner
			let screen = match ::metadevs::video::get_display_for_pos(*pos)
				{
				Some(x) => x,
				// TODO: If now off-screen, warp to a visible position (with ~20px leeway)
				None => todo!("update_dims: Handle window moving off display area"),
				};
			// if window is maximised, keep it that way
			if win.flags.lock().maximised
			{
				// Re-maximise
				*pos = screen.pos();
				win.resize(screen.dims());
			}
		}
		// Recalculate all visibilities
		let count = lh.render_order.len();
		lh.recalc_vis_int(count-1);
	}
	
	// TODO: Poke registered callbacks and tell them that the dimensions have changed
}

/// Handle an input event
pub fn handle_input(event: super::input::Event)
{
	// Push event to a FIFO queue (fixed-size)
	// > This method should be interrupt safe
	match S_EVENT_QUEUE.push(event)
	{
	Ok(_) => {},
	Err(event) => log_notice!("Dropping event {:?}, queue full", event),
	}
	// > Prod a worker (e.g. the render thread) in an atomic way
	S_RENDER_REQUEST.post();
}
/// Switch the currently active window group
//#[tag_safe(irq)]
pub fn switch_active(new: usize)
{
	// TODO: I would like to check the validity of this value BEFORE attempting a re-render, but that
	//  would require locking the S_WINDOW_GROUPS vector.
	// - Technically it shouldn't (reading the size is just racy, not unsafe), but representing that is nigh-on
	//   impossible.
	log_log!("Switching to group {}", new);
	S_CURRENT_GROUP.store(new, ::core::atomic::Ordering::Relaxed);
	S_RENDER_REQUEST.post();
}

// Thread that controls compiositing windows to the screen
fn render_thread()
{
	log_debug!("GUI Render Thread started");
	loop
	{
		// Wait for a signal to start a render
		S_RENDER_REQUEST.sleep();
		
		// Check for events
		while let Some(ev) = S_EVENT_QUEUE.pop()
		{
			log_warning!("TODO: Handle input {:?}", ev);
		}
		
		// Render the active window group
		let (grp_idx, grp_ref) = {
			let grp_idx = S_CURRENT_GROUP.load( ::core::atomic::Ordering::Relaxed );
			let wglh = S_WINDOW_GROUPS.lock();
			match wglh.get(grp_idx)
			{
			Some(r) => (grp_idx, r.clone()),
			None => {
				log_log!("Selected group {} invalid, falling back to 0", grp_idx);
				S_CURRENT_GROUP.store(0, ::core::atomic::Ordering::Relaxed);
				(0, wglh[0].clone())
				},
			}
			};
		
		log_debug!("render_thread: Rendering WG {} '{}'", grp_idx, grp_ref.lock().name);
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
				log_trace!("WindowGroup::redraw: {} '{}' dirty={:?}, vis={:?}", winidx, win.name, dirty, vis);
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
	// TODO: This is quite expensive (causes reallocations on each intersecting window), need to look into
	//       making it cheaper somehow.
	// Maybe have it optionally disabled, and do global dirty instead?
	fn recalc_vis_for(&mut self, vis_idx: usize) -> Vec<Rect>
	{
		// Get the area of the screen used by this window
		let win_idx = self.render_order[vis_idx].0;
		let (ref cur_pos, ref cur_win) = self.windows[win_idx];
		let dims = cur_win.buf.read().dims();
		let win_rect = Rect::new_pd(*cur_pos, dims);
		
		// Iterate all windows above to obtain the visibility rect
		let mut vis = vec![ Rect::new_pd(Pos::new(0,0), dims) ];
		for &(win,_) in &self.render_order[ vis_idx+1 .. ]
		{
			let (ref pos, ref win) = self.windows[win];
			if let Some(mut rect) = Rect::new_pd( *pos, win.buf.read().dims() ).intersect(&win_rect)
			{
				rect.pos.x -= cur_pos.x;
				rect.pos.y -= cur_pos.y;
				
				// Quick check - Is there actually an intersection with the visible regions
				if vis.iter().find(|x| x.intersect(&rect).is_some()).is_some()
				{
					// This window falls within the bounds of the current window
					// - For all visible regions, calculate the relative complement
					//   of this rect with respect to the visible regions
					// - I.e. The areas of the visible region which are not obscured by this win
					let mut new_vis = Vec::new();
					for vis_rect in &vis
					{
						new_vis.extend( vis_rect.not_intersect(&rect) );
					}
					vis = new_vis;
				}
			}
		}
		vis
	}
	
	fn show_window(&mut self, idx: usize) {
		if self.get_render_idx(idx).is_some() {
			return ;
		}
		let rect = Rect { pos: self.windows[idx].0, dims: self.windows[idx].1.buf.read().dims() };
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
	
	fn move_window(&mut self, idx: usize, pos: Pos) {
		self.windows[idx].0 = pos;
		self.recalc_vis(idx);
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
	pub fn alloc<T: Into<String>>(name: T) -> WindowGroupHandle {
		let new_group = Arc::new( Mutex::new( WindowGroup {
			name: T::into(name),
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
	
	pub fn create_window<T: Into<String>>(&mut self, name: T) -> WindowHandle {
		// Allocate a new window from the list
		// - Get handle to this window group (ok to lock it)
		let wgh_rc = S_WINDOW_GROUPS.lock()[self.0].clone();
		
		let idx = wgh_rc.lock().windows.insert( (Pos::new(0,0), Arc::new(Window::new(name.into()))) );
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

impl Window
{
	fn new(name: String) -> Window {
		use core::default::Default;
		Window {
			name: name,
			buf: Default::default(),
			title: Default::default(),
			dirty_rects: Default::default(),
			is_dirty: atomic::ATOMIC_BOOL_INIT,
			flags: Default::default(),
		}
	}
	
	fn take_is_dirty(&self) -> bool { self.is_dirty.swap(false, atomic::Ordering::Relaxed) }
	fn mark_dirty(&self) { self.is_dirty.store(true, atomic::Ordering::Relaxed); }
	
	/// Resize the window
	pub fn resize(&self, dim: Dims) {
		// TODO: use something like "try_make_unique" and emit a notice if it needs to clone
		self.buf.write().make_unique().resize(dim);
		*self.dirty_rects.lock() = vec![ Rect::new(0,0, dim.w, dim.h) ];
	}
	
	/// Add an area to the dirty rectangle list
	fn add_dirty(&self, area: Rect)
	{
		let mut lh = self.dirty_rects.lock();
		
		// 1. Search for overlap with existing regions
		for rgn in &*lh
		{
			// Completely contained within other region
			if rgn.contains_rect(&area) {
				return ;
			}
		}
		// 2. Merge with regions that share a side
		// TODO: Abstract this and run until no merges happen?
		for rgn in &mut **lh
		{
			// Same height and vertical position
			if rgn.top() == area.top() && rgn.bottom() == area.bottom() {
				// - Area's righthand edge within region
				if rgn.left() < area.left() && area.left() <= rgn.right() {
					// Expand region rightwards
					assert!(area.right() > rgn.right());
					let delta = area.right() - rgn.right();
					log_trace!("{} + {} exp right {}px", rgn, area, delta);
					rgn.dims.w += delta;
					return ;
				}
				// - Area's lefthand edge within region
				else if rgn.left() < area.right() && area.right() <= rgn.right() {
					// Expand region leftwards
					assert!(area.left() > rgn.left());
					let delta = area.left() - rgn.left();
					log_trace!("{} + {} exp left {}px", rgn, area, delta);
					rgn.pos.x -= delta;
					rgn.dims.w += delta;
					return ;
				}
			}
			
			// Same width and horizontal position
			if rgn.left() == area.left() && rgn.right() == area.right() {
				// - Area's top edge within region
				if rgn.top() < area.top() && area.top() <= rgn.bottom() {
					// Expand region downwards
					assert!(area.bottom() > rgn.bottom());
					let delta = area.bottom() - rgn.bottom();
					log_trace!("{} + {} exp down {}px", rgn, area, delta);
					rgn.dims.h += delta;
					return ;
				}
				// - Area's bottom edge within region
				else if rgn.top() < area.bottom() && area.bottom() <= rgn.bottom() {
					// Expand region upwards
					assert!(area.top() > rgn.top());
					let delta = area.top() - rgn.top();
					log_trace!("{} + {} exp up {} px", rgn, area, delta);
					rgn.pos.y -= delta; 
					rgn.dims.h += delta;
					return ;
				}
			}
		}
		lh.push(area);
	}
	
	/// Fill an area of the window
	pub fn fill_rect(&self, area: Rect, colour: Colour)
	{
		let buf_h = self.buf.read();
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
	
	/// Set a pixel
	pub fn pset(&self, pos: Pos, colour: Colour)
	{
		self.buf.read().fill_scanline(pos.y as usize, pos.x as usize, 1, colour);
		self.add_dirty( Rect::new_pd(pos, Dims::new(1,1)) );
	}
	
	/// Blit from an external data source
	pub fn blit_rect(&self, rect: Rect, data: &[u32])
	{
		log_trace!("Window::blit_rect({}, data={}px)", rect, data.len());
		let buf_h = self.buf.read();
		for (row,src) in (rect.top() .. rect.bottom()).zip( data.chunks(rect.w() as usize) )
		{
			buf_h.set_scanline(
				row as usize,
				rect.left() as usize,
				src.len(),
				src
				);
		}
		self.add_dirty( rect );
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
	
	//fn get_win_w_pos(&self) -> (Pos, Arc<Window>) {
	//	let wg = self.get_wg();
	//	let wgl = wg.lock();
	//	let win = &wgl.windows[self.win];
	//	(win.0, win.1.clone())
	//}
	
	/// Obtain a reference to the window's backing buffer
	///
	/// This is invalidated (no longer backs the window) if the window is
	/// resized.
	pub fn get_buffer(&self) -> Arc<WinBuf> {
		let win = self.get_win();
		let lh = win.buf.read();
		lh.clone()
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
		let wg = self.get_wg();
		wg.lock().recalc_vis(self.win);
	}
	pub fn set_pos(&mut self, pos: Pos) {
		let wg = self.get_wg();
		wg.lock().move_window(self.win, pos);
	}
	
	/// Maximise this window (fill all space on the current monitor)
	pub fn maximise(&mut self) {
		let win = self.get_win();
		win.flags.lock().maximised = true;
		let wg = self.get_wg();
		wg.lock().maximise_window( self.win );
		// No need to call trigger_recalc_vis, maximise_window does that
	}
	/// Show the window
	pub fn show(&mut self) {
		let wg = self.get_wg();
		wg.lock().show_window( self.win );
		self.redraw();
	}
	/// Hide the window
	#[allow(dead_code)]
	pub fn hide(&mut self) {
		let wg = self.get_wg();
		wg.lock().hide_window( self.win );
		self.redraw();
	}
	
	/// Fill an area of the window with a specific colour
	pub fn fill_rect(&mut self, area: Rect, colour: Colour)
	{
		self.get_win().fill_rect(area, colour);
	}
	
	/// Set single pixel (VERY inefficient, don't use unless you need to)
	pub fn pset(&mut self, pos: Pos, colour: Colour)
	{
		self.get_win().pset(pos, colour);
	}
	
	/// Fill a region of the window with provided data
	pub fn blit_rect(&mut self, rect: Rect, data: &[u32])
	{
		self.get_win().blit_rect(rect, data);
	}
}

impl ::core::ops::Drop for WindowHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

