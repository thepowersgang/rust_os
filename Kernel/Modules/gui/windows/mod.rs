// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/windows/mod.rs
// - GUI Window management
use kernel::prelude::*;
use super::{Dims,Pos,Rect,Colour};
use kernel::sync::mutex::{LazyMutex,Mutex};
use kernel::lib::mem::Arc;
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use kernel::lib::LazyStatic;
use core::sync::atomic;

use kernel::lib::sparse_vec::SparseVec;

pub use self::winbuf::WinBuf;

type WinId = u16;
type GrpId = u16;

/// Handle to the backing buffer of a window
pub type BufHandle = Arc<WinBuf>;

mod window;
mod winbuf;

use self::window::Window;

/// Window groups combine windows into "sessions", that can be switched with magic key combinations
struct WindowGroup
{
	/// Number of active handles to this window group
	refcount: u32,

	/// Window group name, may be shown to the user if requested
	name: String,

	/// Window that currently has focus, different to the top of the render order
	focussed_window: WinId,

	/// Canonical list of windows (sparse, for reallocation of IDs)
	///
	/// Contains both the window position and shared ownership of the window data.
	/// Position is here because the window itself doesn't need control (or knowledge) of its position
	windows: SparseVec< (Pos, Aref<Window>) >,
	/// Render order (indexes into `windows`, and visibilities)
	render_order: Vec< (WinId, Vec<Rect>) >,
}


/// Handle on a window group (owning, when dropped the group is destroyed)
pub struct WindowGroupHandle(GrpId);

/// Window handle (when dropped, the window is destroyed)
pub struct WindowHandle
{
	grp: Arc<Mutex<WindowGroup>>,
	win: Option<ArefBorrow<Window>>,
	grp_id: GrpId,
	win_id: WinId,
}

#[derive(Default)]
struct CursorPos {
	old_x: u32,
	old_y: u32,
	new_x: u32,
	new_y: u32,
	is_dirty: bool,
}

// - 13 sessions, #0 is fixed to be the kernel's log 1-12 are bound to F1-F12
const C_MAX_SESSIONS: usize = 13;
static S_WINDOW_GROUPS: LazyMutex<SparseVec< Arc<Mutex<WindowGroup>> >> = lazymutex_init!();
static S_CURRENT_GROUP: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);

static S_RENDER_REQUEST: ::kernel::sync::EventChannel = ::kernel::sync::EventChannel::new();
static S_RENDER_NEEDED: atomic::AtomicBool = atomic::AtomicBool::new(false);
static S_FULL_REDRAW: atomic::AtomicBool = atomic::AtomicBool::new(false);
static S_EVENT_QUEUE: LazyStatic<::kernel::lib::ring_buffer::AtomicRingBuf<super::input::Event>> = lazystatic_init!();
static S_MOVE_STATE: Mutex<CursorPos> = Mutex::new(CursorPos::new());
// Keep this lazy, as it's runtime initialised
static S_RENDER_THREAD: LazyMutex<::kernel::threads::WorkerThread> = lazymutex_init!();

pub fn init()
{
	S_WINDOW_GROUPS.init( || SparseVec::new() );
	
	// Create render thread
	S_EVENT_QUEUE.prep(|| ::kernel::lib::ring_buffer::AtomicRingBuf::new(32));
	S_RENDER_THREAD.init( || ::kernel::threads::WorkerThread::new("GUI Compositor", render_thread) );
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
			// if window is maximised, keep it that way
			if win.flags.lock().maximised
			{
				// Locate screen for the upper-left corner
				let screen = match ::kernel::metadevs::video::get_display_for_pos(*pos)
					{
					Ok(x) => x,
					// TODO: If now off-screen, warp to a visible position (with ~20px leeway)
					Err(r) => {
						todo!("update_dims: Handle full-screen window moving off display area - {:?} - {:?}", *pos, r)
						},
					};
				// Re-maximise
				*pos = screen.pos();
				win.resize(screen.dims());
			}
			else
			{
				// Otherwise, ensure that the window stays visible
				match ::kernel::metadevs::video::get_display_for_pos(*pos)
				{
				Ok(r) => {
					// TODO: Crop window's display area to fit on-screen?
					log_debug!("Check {:?} vs {:?}", *pos, r);
					},
				// If now off-screen, warp to a visible position (with ~20px leeway)
				Err(r) => {
					const MIN_VISIBLE: u32 = 20;
					log_debug!("update_dims: Warping window from {:?} to 20px within {:?}", *pos, r);
					pos.x = ::core::cmp::min(pos.x, r.right() - MIN_VISIBLE);
					pos.x = ::core::cmp::max(pos.x, r.left());
					pos.y = ::core::cmp::min(pos.y, r.bottom() - MIN_VISIBLE);
					pos.y = ::core::cmp::max(pos.y, r.top());
					},
				}
			}
		}
		// Recalculate all visibilities
		let count = lh.render_order.len();
		lh.recalc_vis_int(count-1);
	}
	
	// TODO: Poke registered callbacks and tell them that the dimensions have changed

	// Force a full redraw
	S_RENDER_NEEDED.store(true, atomic::Ordering::Relaxed);
	S_FULL_REDRAW.store(true, atomic::Ordering::Relaxed);
	S_RENDER_REQUEST.post();
}

/// Handle an input event
//#[tag_safe(irq)]
pub fn handle_input(event: super::input::Event)
{
	// Push event to a FIFO queue (fixed-size)
	// > Queue is cleared by the render thread
	// > This method should be interrupt safe
	match event
	{
	super::input::Event::MouseMove(x,y, _dx,_dy) => {
		S_MOVE_STATE.lock().update( x, y );
		},
	event @ _ =>
		match S_EVENT_QUEUE.push(event)
		{
		Ok(_) => {},
		Err(event) => log_notice!("Dropping event {:?}, queue full", event),
		},
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
	S_CURRENT_GROUP.store(new as usize, atomic::Ordering::Relaxed);
	S_RENDER_NEEDED.store(true, atomic::Ordering::Relaxed);
	S_FULL_REDRAW.store(true, atomic::Ordering::Relaxed);
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
		
		// Render the active window group
		let (grp_idx, grp_ref) = {
			let grp_idx = S_CURRENT_GROUP.load( atomic::Ordering::Relaxed );
			let wglh = S_WINDOW_GROUPS.lock();
			match wglh.get(grp_idx)
			{
			Some(r) => {
				(grp_idx, r.clone())
				},
			None if grp_idx == 0 => {
				log_notice!("Group 0 invalid, sleeping render thread");
				continue;
				},
			None => {
				log_log!("Selected group {} invalid, falling back to 0", grp_idx);
				S_CURRENT_GROUP.store(0, atomic::Ordering::Relaxed);
				(0, wglh[0].clone())
				},
			}
			};
		
		// Check for events
		// TODO: Could this be moved into the `handle_input()` function?
		while let Some(ev) = S_EVENT_QUEUE.pop()
		{
			// - TODO: Filter out global bindings (e.g. session switch and lock combos)
			//  > NOTE: Session switch is currently handled by the input code
			// - Just pass on to active group
			grp_ref.lock().handle_input(ev);
		}
		// - If the mouse has moved since last trigger, pass that on to the group
		if let Some( (x,y, dx,dy) ) = S_MOVE_STATE.lock().take() {
			grp_ref.lock().handle_input( super::input::Event::MouseMove(x,y, dx,dy) );
		}
		
		if S_RENDER_NEEDED.swap(false, atomic::Ordering::Relaxed)
		{
			log_debug!("render_thread: Rendering WG {} '{}'", grp_idx, grp_ref.lock().name);
			grp_ref.lock().redraw( S_FULL_REDRAW.swap(false, atomic::Ordering::Relaxed) );
		}
	}
}

impl WindowGroup
{
	fn new(name: String) -> WindowGroup {
		WindowGroup {
			refcount: 1,
			name: name,
			focussed_window: 0,
			windows: SparseVec::new(),
			render_order: Vec::new(),
			}
	}
	/// Increment the reference count
	fn inc_ref(&mut self) {
		assert!(self.refcount < !0);
		self.refcount += 1;
	}
	/// Decrement the handle reference count
	///
	/// Returns `true` if the reference count reaches zero
	fn deref(&mut self) -> bool
	{
		assert!(self.refcount > 0);
		self.refcount -= 1;
		if self.refcount == 0 {
			// Delete all windows
			self.focussed_window = 0;
			self.render_order.truncate(0);
			// - Can't drop all the windows yet, their handles include ArefBorrow-s
			//self.windows = Default::default();
			true
		}
		else {
			false
		}
	}

	/// Re-draw this window group
	fn redraw(&mut self, full: bool)
	{
		log_trace!("WindowGroup::redraw: render_order={:?}", self.render_order);
		for &(winidx,ref vis) in &self.render_order
		{
			let vis = &vis[..];
			let (ref pos, ref win) = self.windows[winidx as usize];
			// 1. Is the window dirty, or are we doing a full redraw
			if win.take_is_dirty() || full
			{
				static FULL_RECT: [Rect; 1] = [Rect { pos: Pos {x:0,y:0},dims: Dims{w:!0,h:!0}}];
				
				// 2. Obtain the visible sections of this window that have changed
				// - Switch the dirty rect out with an empty Vec
				let dirty_vec = win.take_dirty_rects();
				// - Get a slice of it (OR, if doing a full re-render, get a wildcard region)
				let dirty = if full { &FULL_RECT[..] } else { &dirty_vec[..] };
				log_trace!("WindowGroup::redraw: {} '{}' dirty={:?}, vis={:?}", winidx, win.name(), dirty, vis);
				// - Iterate all visible dirty regions and re-draw
				for rgn in Rect::list_intersect(vis, dirty)
				{
					// Blit data from the window to the screen
					win.blit_rgn_to_screen(*pos, rgn);
				}
			}
		}
	}
	
	fn get_win_at_pos(&self, x: u32, y: u32) -> Option<&(Pos, Aref<Window>)> {
		let mut rv = None;
		// Iterate render order finding the highest (latest) window which contains this point
		for &(winidx, _) in &self.render_order
		{
			let ptr = &self.windows[winidx as usize];
			let &(pos, ref win) = ptr;
			let dims = win.dims();

			if pos.x <= x && pos.y <= y {
				if x < pos.x + dims.w && y < pos.y + dims.h {
					rv = Some(ptr);
				}
			}
		}
		log_trace!("rv = {:?}", rv);
		rv
	}

	fn handle_input(&mut self, ev: super::input::Event) {
		use super::input::Event;
		match ev
		{
		Event::KeyDown(..) | Event::KeyUp(..) | Event::KeyFire(..) | Event::Text(..) => {
			// - Apply shortcuts defined by the current session (TODO)
			// - Pass events to the current window
			if let Some(_) = self.get_render_idx( self.focussed_window )
			{
				match self.windows.get( self.focussed_window as usize )
				{
				Some(w) => w.1.handle_input(ev),
				None => log_log!("Active window #{} not present", self.focussed_window),
				}
			}
			else {
				self.focussed_window = 0;
			}
			},
		Event::MouseMove(x,y, dx,dy) =>
			if let Some(newwin) = self.get_win_at_pos(x,y)
			{
				//if self.mouse_last_win != &newwin {
				//}
				let Pos { x: bx, y: by } = newwin.0;
				newwin.1.handle_input( Event::MouseMove(x - bx, y - by, dx, dy) );
			}
			else
			{
				//if !self.mouse_last_win.is_null() {
				//}
			},
		Event::MouseDown(x,y, btn) =>
			if let Some(newwin) = self.get_win_at_pos(x,y)
			{
				//self.mouse_down_win = &newwin;
				let Pos { x: bx, y: by } = newwin.0;
				newwin.1.handle_input( Event::MouseDown(x - bx, y - by, btn) );
			},
		Event::MouseClick(x,y, btn, count) =>
			if let Some(newwin) = self.get_win_at_pos(x,y)
			{
				//self.mouse_down_win = &newwin;
				let Pos { x: bx, y: by } = newwin.0;
				newwin.1.handle_input( Event::MouseClick(x - bx, y - by, btn, count) );
			},
		Event::MouseUp(x,y, btn) =>
			if let Some(newwin) = self.get_win_at_pos(x,y)
			{
				//if self.mouse_down_win != &newwin {
				//}
				let Pos { x: bx, y: by } = newwin.0;
				newwin.1.handle_input( Event::MouseUp(x - bx, y - by, btn) );
			}
			else
			{
				//if !self.mouse_down_win.is_null() {
				//}
			},
		}
	}

	/// Obtains the render position of the specified window
	fn get_render_idx(&self, winidx: WinId) -> Option<usize> {
		self.render_order.iter().position( |&(idx,_)| idx == winidx )
	}
		
	/// Recalculate the cached visibility regions caused by 'changed_idx' updating
	fn recalc_vis(&mut self, changed_idx: WinId)
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
		if ! self.render_order.is_empty()
		{
			for i in (0 .. vis_idx+1).rev()
			{
				// Iterate all higher windows and obtain visible rects
				self.render_order[i].1 = self.recalc_vis_for(i);
			}
		}
	}
	/// Recalculate the visibility vector for a specific window in the render order
	// TODO: This is quite expensive (causes reallocations on each intersecting window), need to look into
	//	   making it cheaper somehow.
	// Maybe have it optionally disabled, and do global dirty instead?
	fn recalc_vis_for(&mut self, vis_idx: usize) -> Vec<Rect>
	{
		// Get the area of the screen used by this window
		let win_idx = self.render_order[vis_idx].0;
		let (ref cur_pos, ref cur_win) = self.windows[win_idx as usize];
		let dims = cur_win.dims();
		let win_rect = Rect::new_pd(*cur_pos, dims);
		
		// Iterate all windows above to obtain the visibility rect
		let mut vis = vec![ Rect::new_pd(Pos::new(0,0), dims) ];
		for &(win,_) in &self.render_order[ vis_idx+1 .. ]
		{
			let (ref pos, ref win) = self.windows[win as usize];
			if let Some(mut rect) = Rect::new_pd( *pos, win.dims() ).intersect(&win_rect)
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
	
	fn show_window(&mut self, idx: WinId) {
		if self.get_render_idx(idx).is_some() {
			return ;
		}
		let rect = Rect { pos: self.windows[idx as usize].0, dims: self.windows[idx as usize].1.dims() };
		self.render_order.push( (idx, vec![rect]) );
		let vis_idx = self.render_order.len() - 1;
		self.recalc_vis_int(vis_idx);

		// TODO: Have a better method than just switching focus on show
		self.focussed_window = idx;
	}
	fn hide_window(&mut self, idx: WinId) {
		if let Some(pos) = self.get_render_idx(idx)
		{
			let prev_pos = if pos == 0 { 0 } else { pos - 1 };
			self.render_order.remove(pos);
			// If this window was the focussed one, switch to the next lower down window
			// - TODO: Have an alt-tab order and use that instead
			if self.focussed_window == idx {
				self.focussed_window = self.render_order.get( prev_pos ).map(|x| x.0).unwrap_or(0);
			}
			// Recalculate visibility for lower window
			self.recalc_vis_int(prev_pos);

			// TODO: Full redraw can be expensive... would prefer to force redraw of just the revealed region
			S_FULL_REDRAW.store(true, atomic::Ordering::Relaxed);
			S_RENDER_NEEDED.store(true, atomic::Ordering::Relaxed);
			S_RENDER_REQUEST.post();
		}
		else {
			log_debug!("Window {} not visible", idx);
		}
	}
	
	fn move_window(&mut self, idx: WinId, pos: Pos) {
		self.windows[idx as usize].0 = pos;
		self.recalc_vis(idx);
	}
	fn get_window_pos(&self, idx: WinId) -> Pos {
		self.windows[idx as usize].0
	}
	
	fn maximise_window(&mut self, idx: WinId) {
		{
			let &mut(ref mut pos, ref win_rc) = &mut self.windows[idx as usize];
			let rect = match ::kernel::metadevs::video::get_display_for_pos(*pos)
				{
				Ok(x) => x,
				Err(r) => {
					log_error!("TODO: Handle window being off-screen - closest {:?}", r);
					Rect::new(0,0, 0,0)
					},
				};
			// - Move window to new position
			*pos = rect.pos();
			// - Resize window
			win_rc.resize(rect.dims());
		}
		// Recalculate visible regions
		self.recalc_vis(idx);
	}


	/// Drops (functionally destroys) a window
	fn drop_window(&mut self, idx: WinId) {
		self.hide_window(idx);
		self.windows.remove(idx as usize);
	}
}

impl WindowGroupHandle
{
	pub fn alloc<T: Into<String>>(name: T) -> WindowGroupHandle {
		let new_group = Arc::new( Mutex::new( WindowGroup::new(name.into()) ) );
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
		WindowGroupHandle(idx as GrpId)
	}

	fn with_wg<R, F: FnOnce(&mut WindowGroup)->R>(&self, fcn: F) -> R {
		let wgs = S_WINDOW_GROUPS.lock();
		let mut wgh = wgs[self.0 as usize].lock();
		fcn( &mut wgh )
	}
	
	pub fn create_window<T: Into<String>>(&mut self, name: T) -> WindowHandle {
		// Allocate a new window from the list
		// - Get handle to this window group (ok to lock it)
		let wgh_rc = S_WINDOW_GROUPS.lock()[self.0 as usize].clone();
		
		let win = Aref::new(Window::new(name.into()));
		let win_ref = win.borrow();
		let idx = wgh_rc.lock().windows.insert( (Pos::new(0,0), win) );
		WindowHandle { win: Some(win_ref), grp: wgh_rc, grp_id: self.0, win_id: idx as WinId }
	}

	/// Force this group to be the active group
	pub fn force_active(&self) {
		switch_active(self.0 as usize);
	}
}
impl Clone for WindowGroupHandle
{
	fn clone(&self) -> WindowGroupHandle {
		self.with_wg(|wg| wg.inc_ref());
		WindowGroupHandle( self.0 )
	}
}
impl ::core::ops::Drop for WindowGroupHandle
{
	fn drop(&mut self)
	{
		if self.with_wg(|wg| wg.deref()) == true {
			log_notice!("Window group {} destroyed", self.0);
			S_WINDOW_GROUPS.lock().remove(self.0 as usize);
			switch_active(0);
		}
	}
}

impl WindowHandle
{
	fn get_win(&self) -> &Window {
		self.win.as_ref().unwrap()
	}
	
	//fn get_win_w_pos(&self) -> (Pos, Arc<Window>) {
	//	let wgl = self.grp.lock();
	//	let win = &wgl.windows[self.win];
	//	(win.0, win.1.clone())
	//}
	
	/// Obtain a reference to the window's backing buffer
	///
	/// This is invalidated (no longer backs the window) if the window is
	/// resized.
	pub fn get_buffer(&self) -> Arc<WinBuf> {
		self.get_win().get_buffer()
	}
	
	/// Redraw the window (mark for re-blitting)
	pub fn redraw(&self)
	{
		// if shown, mark self as requiring reblit and poke group
		if self.grp_id as usize == S_CURRENT_GROUP.load(atomic::Ordering::Relaxed)
		{
			self.get_win().mark_dirty();
			S_RENDER_NEEDED.store(true, atomic::Ordering::Relaxed);
			S_RENDER_REQUEST.post();
		}
	}
	
	/// Resize the window
	pub fn resize(&mut self, dim: Dims) {
		self.get_win().resize(dim);
		self.grp.lock().recalc_vis(self.win_id);
	}
	pub fn set_pos(&mut self, pos: Pos) {
		self.grp.lock().move_window(self.win_id, pos);
	}

	/// Return the dimensions of the currently usable portion of the window
	pub fn get_dims(&self) -> Dims {
		self.get_win().dims()
	}
	pub fn get_pos(&self) -> Pos {
		let rv = self.grp.lock().get_window_pos(self.win_id);
		rv
	}
	
	/// Maximise this window (fill all space on the current monitor)
	pub fn maximise(&mut self) {
		let win = self.get_win();
		win.flags.lock().maximised = true;
		self.grp.lock().maximise_window( self.win_id );
		// No need to call trigger_recalc_vis, maximise_window does that
	}
	/// Show the window
	pub fn show(&mut self) {
		self.grp.lock().show_window( self.win_id );
		self.redraw();
	}
	/// Hide the window
	pub fn hide(&mut self) {
		self.grp.lock().hide_window( self.win_id );
		self.redraw();
	}
	
	/// Fill an area of the window with a specific colour
	pub fn fill_rect(&mut self, area: Rect, colour: Colour) {
		self.get_win().fill_rect(area, colour);
	}
	
	/// Fill a region of the window with provided data
	pub fn blit_rect(&mut self, rect: Rect, data: &[u32], stride: usize) {
		self.get_win().blit_rect(rect, data, stride);
	}

	pub fn pop_event(&self) -> Option<super::input::Event> {
		self.get_win().input.pop_event()
	}
	pub fn bind_wait_input(&self, obj: &mut ::kernel::threads::SleepObject) {
		self.get_win().input.bind_wait(obj);
	}
	pub fn clear_wait_input(&self, obj: &mut ::kernel::threads::SleepObject) {
		self.get_win().input.clear_wait(obj);
	}
}

impl ::core::ops::Drop for WindowHandle
{
	fn drop(&mut self)
	{
		log_debug!("WindowHandle::drop - {}/{}", self.grp_id, self.win_id);
		// WindowHandle uniquely owns the window, so can just drop it
		self.win = None;
		self.grp.lock().drop_window( self.win_id );
	}
}

impl CursorPos
{
	const fn new() -> CursorPos {
		CursorPos {
			old_x: 0, old_y: 0,
			new_x: 0, new_y: 0,
			is_dirty: false
		}
	}
	fn update(&mut self, x: u32, y: u32) {
		self.new_x = x;
		self.new_y = y;
		self.is_dirty = true;
		log_debug!("CursorPos::update - ({},{})", x,y);
	}
	fn is_dirty(&self) -> bool {
		self.is_dirty
	}
	fn take(&mut self) -> Option<(u32,u32, i16,i16)> {
		if self.is_dirty()
		{
			let dx = (self.new_x as i32 - self.old_x as i32) as i16;
			let dy = (self.new_y as i32 - self.old_y as i32) as i16;
			self.is_dirty = false;
			self.old_x = self.new_x;
			self.old_y = self.new_y;
			let rv = (self.new_x, self.new_y,  dx, dy);
			log_debug!("CursorPos::take - {:?}", rv);
			Some( rv )
		}
		else
		{
			None
		}
	}
}

