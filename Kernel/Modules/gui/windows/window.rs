// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/windows/window.rs
//! Window type and helpers
use kernel::prelude::*;

use kernel::sync::rwlock::RwLock;
use kernel::sync::mutex::Mutex;
use kernel::lib::mem::Arc;
use kernel::lib::ring_buffer::{RingBuf};
use core::sync::atomic;

use super::winbuf::WinBuf;
use ::{Dims,Pos,Rect,Colour};
use input;

/// A single window, an arbitrarily movable on-screen region
//#[derive(Default)]
pub struct Window
{
	/// Window name (set once, never changes after)
	// TODO: Box<str>
	name: String,
	
	/// Actual window data
	/// 
	/// Write lock is for structure manipulations, slicing is sharable.
	/// Arc allows the "user" to hold a copy of the framebuffer
	buf: RwLock<Arc<WinBuf>>,

	/// List of invalidated regions within the window
	dirty_rects: Mutex<Vec<Rect>>,
	is_dirty: atomic::AtomicBool,
	
	/// Flags on the window
	pub flags: Mutex<WindowFlags>,

	/// Input channel
	pub input: WindowInput,
}
impl_fmt! {
	Debug(self, f) for Window {
		write!(f, "Window({:?}, {:?})", self.name, self.buf.read().dims())
	}
}
pub struct WindowInput {
	queue: Mutex<RingBuf<input::Event>>,
	cursor: Mutex<super::CursorPos>,
	waiters: ::kernel::user_async::Queue,
}

#[derive(Default)]
pub struct WindowFlags
{
	/// If true, the window is maximised, and should be resized with the screen
	pub maximised: bool,
}


impl Window
{
	pub fn new(name: String) -> Window {
		Window {
			name: name,
			buf: Default::default(),
			dirty_rects: Default::default(),
			is_dirty: atomic::AtomicBool::new(false),
			flags: Default::default(),
			input: WindowInput {
				queue: Mutex::new(RingBuf::new(16)),
				waiters: Default::default(),
				cursor: Default::default(),
				},
		}
	}
	pub fn name(&self) -> &str {
		&self.name
	}
	pub fn dims(&self) -> Dims {
		self.buf.read().dims()
	}
	pub fn get_buffer(&self) -> Arc<WinBuf> {
		self.buf.read().clone()
	}

	pub fn take_dirty_rects(&self) -> Vec<Rect> {
		::core::mem::replace(&mut *self.dirty_rects.lock(), Vec::new())
	}
	pub fn take_is_dirty(&self) -> bool {
		self.is_dirty.swap(false, atomic::Ordering::Relaxed)
	}
	pub fn mark_dirty(&self) {
		self.is_dirty.store(true, atomic::Ordering::Relaxed);
	}


	pub fn handle_input(&self, ev: input::Event) {
		// TODO: Filter events according to a map
		self.input.push_event(ev);
	}

	/// Resize the window
	pub fn resize(&self, dim: Dims) {
		// TODO: use something like "try_make_unique" and emit a notice if it needs to clone
		//Arc::get_mut(&mut self.buf.write())
		Arc::make_mut(&mut self.buf.write()).resize(dim);
		*self.dirty_rects.lock() = vec![ Rect::new(0,0, dim.w, dim.h) ];
		self.input.push_event(input::Event::Resize);
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
		let dims = self.buf.read().dims();
		let winrect = Rect::new_pd(Pos::new(0,0), dims);
		if let Some(area) = area.intersect(&winrect)
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
	}
	
	/// Blit from an external data source
	/// 
	/// `stride` is the number of data entries between rows (allows inner blits)
	pub fn blit_rect(&self, area: Rect, data: &[u32], stride: usize)
	{
		log_trace!("Window::blit_rect({}, data={}px)", area, data.len());
		let buf_h = self.buf.read();
		for (row,src) in (area.top() .. area.bottom()).zip( data.chunks(stride) )
		{
			buf_h.set_scanline(
				row as usize,
				area.left() as usize,
				src.len(),
				src
				);
		}
		self.add_dirty( area );
	}


	pub fn blit_rgn_to_screen(&self, pos: Pos, rgn: Rect) {
		self.buf.read().blit_to_display(pos, rgn);
	}
}


impl WindowInput
{
	pub fn push_event(&self, ev: input::Event)
	{
		// TODO: Coalesce mouse movement events
		match ev
		{
		input::Event::MouseMove(x,y, _dx,_dy) => {
			self.cursor.lock().update(x,y)
			},
		ev @ _ => {
			let _ = self.queue.lock().push_back(ev);	// silently drop extra events?
			},
		}
		self.waiters.wake_one();
	}

	pub fn pop_event(&self) -> Option<input::Event> {
		let mut lh = self.queue.lock();
		let rv = lh.pop_front();
		if ! lh.is_empty() {
			self.waiters.wake_one();
		}
		if rv.is_some() {
			rv
		}
		else {
			self.cursor.lock().take()
				.map( |(x,y, dx,dy)| input::Event::MouseMove(x, y, dx, dy) )
		}
	}
	pub fn bind_wait(&self, obj: &mut ::kernel::threads::SleepObject) {
		self.waiters.wait_upon(obj);
		if ! self.queue.lock().is_empty() || self.cursor.lock().is_dirty() {
			obj.signal()
		}
	}
	pub fn clear_wait(&self, obj: &mut ::kernel::threads::SleepObject) {
		self.waiters.clear_wait(obj);
	}
}
