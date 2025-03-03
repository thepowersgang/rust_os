// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/kernel_log.rs
/// Kernel log output (and debug terminal)
//
// Manages a set of windows giving a view into the kernel
// - Kernel log (current) : Contains the most recent kernel log messages
// - Logo
// - TODO: Kernel log (history) : A searchable/filterable/scrollable kernel log
// - TODO: Console 
//
// DESIGN NOTES
// - Option 1: Store just one screen worth, rendering when the log is generated
//   - Minimum memory usage (just the framebuffer)
//   - Some quirks getting the buffer while still handling resizes
// - Option 2: Store ~1MiB text (ring?) buffer, and render out of it
//   - Challenge: EventChannel::post may not be callable from logging?
//   - Could restructure how the kernel log works - so it pokes the non-immediate sinks after releasing its main lock
#[allow(unused_imports)]
use kernel::prelude::*;
use ::core::sync::atomic::Ordering;

use super::windows::{WindowGroupHandle,WindowHandle};
use super::{Colour,Dims,Pos,Rect};
use core::fmt;
use kernel::sync::mutex::LazyMutex;

// Bitmap font used by this module is in another file
use ::embedded_images::font_cp437_8x16::{S_FONTDATA,unicode_to_cp437};

// Raw bitmap logo (already encoded with dims and as a rust array)
use ::embedded_images::logo;

struct KernelLog
{
	_wgh: WindowGroupHandle,
	logo_wh: WindowHandle,
	main_wh: WindowHandle,
}

/// Character position
/// NOTE: This is `(y,x)`
#[derive(Copy,Clone,Debug)]
struct CharPos(u32,u32);

struct LogWriter
{
	pos: CharPos,
	colour: Colour,
	no_flush: bool,
}

/// Trait to provde 'is_combining', used by render code
trait UnicodeCombining
{
	fn is_combining(&self) -> bool;
}

const C_CELL_DIMS: Dims = Dims { w: 8, h: 16 };
static S_KERNEL_LOG: ::kernel::sync::OnceCell<KernelLog> = ::kernel::sync::OnceCell::new();
static S_BUFFER_HANDLE: tmp::AtomicArc<crate::windows::WinBuf> = tmp::AtomicArc::new();
static S_CURRENT_LINE: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new(0);

#[doc(hidden)]
pub fn init()
{
	// Create window (and structure)
	let kl = KernelLog::new();
	S_KERNEL_LOG.get_init(|| kl);

	// TODO: Make a worker thread that handles input and reloads the buffer on resize

	{
		static S_WORKER: LazyMutex<::kernel::threads::WorkerThread> = lazymutex_init!();
		S_WORKER.init( || ::kernel::threads::WorkerThread::new("GUI KernelLog", || {
			// SAFE: This object outlives anything that borrows it
			let mut so = unsafe { ::kernel::threads::SleepObject::new("GUI KernelLog") };
			let kl = S_KERNEL_LOG.get();
			loop {
				kl.main_wh.bind_wait_input(&mut so);
				kl.logo_wh.bind_wait_input(&mut so);
				so.wait();
				while let Some(v) = kl.main_wh.pop_event().or_else(|| kl.logo_wh.pop_event()) {
					use crate::input::Event;
					match v {
					Event::Resize => {
						let bh = kl.main_wh.get_buffer();
						//log_trace!("Update bh={:p}", bh);
						let max_dims = bh.dims();
						S_BUFFER_HANDLE.set( bh );
						kl.logo_wh.set_pos(Pos::new(max_dims.w-kl.logo_wh.get_dims().w, 0));
					},
					Event::KeyDown(_) => {},
					Event::KeyUp(_) => {},
					Event::KeyFire(key_code) => {
						use ::key_codes::KeyCode;
						// TODO: Take actions (e.g. scrolling view)
						match key_code {
						KeyCode::Return => {},
						_ => {},
						}
					},
					Event::Text(translated_key) => {
						// TODO: Put into a kernel prompt
					},
					Event::MouseMove(_, _, _, _)
					|Event::MouseDown(_, _, _)
					|Event::MouseUp(_, _, _)
					|Event::MouseClick(_, _, _, _) => {},
					}
				}
				kl.main_wh.clear_wait_input(&mut so);
				kl.logo_wh.clear_wait_input(&mut so);
			}
			}));
	}

	{
		use core::fmt::Write;
		write!(&mut LogWriter::new(Colour::def_green() ), "{}", ::kernel::build_info::version_string()).unwrap();
		write!(&mut LogWriter::new(Colour::def_yellow()), "> {}", ::kernel::build_info::build_string()).unwrap();
	}
	
	// TODO: Populate kernel logging window with accumulated logs

	// Register to receive logs
	::kernel::logging::register_gui(LogHandler::default());
}

#[derive(Default)]
struct LogHandler
{
	inner: Option<LogWriter>,
}
impl ::kernel::logging::Sink for LogHandler {
	fn start(&mut self, timestamp: kernel::time::TickCount, level: kernel::logging::Level, source: &'static str) {
		use ::kernel::logging::Level;
		let c = match level {
			Level::Panic   => Colour(0xFF00FF),
			Level::Error   => Colour::def_red(),
			Level::Warning => Colour::def_yellow(),
			Level::Notice  => Colour::def_green(),
			Level::Info    => Colour::def_blue(),
			Level::Log	   => Colour::def_white(),
			Level::Debug   => Colour::def_white(),
			Level::Trace   => Colour::def_gray(),
			};
		self.inner = Some(LogWriter::new(c));
		let i = self.inner.as_mut().unwrap();
		i.no_flush |= source.starts_with("gui::");

		use ::core::fmt::Write;
		write!(i, "{:6}{} {}/{}[{}] - ", timestamp, level, ::kernel::arch::cpu_num(), ::kernel::threads::get_thread_id(), source).unwrap();
	}

	fn write(&mut self, data: &str) {
		let _ = ::core::fmt::Write::write_str(self.inner.as_mut().unwrap(), data);
	}

	fn end(&mut self) {
		self.inner = None;
	}
}

impl KernelLog
{
	fn new() -> KernelLog
	{
		// Kernel's window group	
		let mut wgh = WindowGroupHandle::alloc("Kernel");
		
		// - Log Window
		let mut wh = wgh.create_window("Kernel Log");
		wh.maximise();
		let log_win_buf = wh.get_buffer();
		
		// - Fancy logo window
		let dims = Dims::new(logo::DIMS.0,logo::DIMS.1);
		let mut logo_wh = wgh.create_window("Logo");
		let max_dims = log_win_buf.dims();
		if max_dims != Dims::new(0,0)
		{
			logo_wh.set_pos(Pos::new(max_dims.w-dims.w, 0));
			logo_wh.resize(dims);
			logo_wh.blit_rect( Rect::new_pd(Pos::new(0,0),dims), &logo::DATA, dims.w as usize );
		}

		S_BUFFER_HANDLE.set( log_win_buf );
			
		if max_dims != Dims::new(0,0)
		{
			// > Show windows in reverse render order
			wh.show();
			logo_wh.show();
		}
		
		// Return struct populated with above handles	
		KernelLog {
			_wgh: wgh,
			main_wh: wh,
			logo_wh,
		}
	}
	
	/// Scroll the display up a step, revealing a new line
	fn reveal_new_line(&self)
	{
		let Some(bh) = S_BUFFER_HANDLE.get() else { return };
		let mut line_no = S_CURRENT_LINE.fetch_add(1, Ordering::Relaxed);
		// NOTE: This should be safe, as this function is only called with the global logging lock held
		while line_no >= bh.dims().h / C_CELL_DIMS.h {
			let n_rows = 1;
			let scroll_px = (n_rows * C_CELL_DIMS.h) as usize;
			let h = bh.dims().h as usize;
			bh.copy_internal(0, scroll_px, 0, 0, bh.dims().w as usize, h - scroll_px);
			for line in h - scroll_px .. h {
				bh.fill_scanline(line, 0, bh.dims().w as usize, Colour::def_gray());
			}
			line_no = S_CURRENT_LINE.fetch_sub(n_rows, Ordering::Relaxed) - n_rows;
		}
	}
	/// Flush changes
	//#[req_safe(taskswitch)]	//< Must be safe to call from within a spinlock
	fn flush(&self)
	{
		// Poke the WM and tell it to reblit us (when it's able to)
		// - This version is safe to call with a spinlock held
		self.main_wh.redraw_lazy();
	}
	
	/// Write a string to the log display (at the given character position)
	fn write_text(&self, mut pos: CharPos, colour: Colour, text: &str) -> CharPos
	{
		let Some(bh) = S_BUFFER_HANDLE.get() else { return pos; };
		if bh.dims().w == 0 || bh.dims().h == 0 {
			return pos;
		}
		for c in text.chars()
		{
			// Refuse to print if the print would go out of bounds
			if pos.0 >= bh.dims().h / C_CELL_DIMS.h || pos.1 >= bh.dims().w / C_CELL_DIMS.w {
				//::kernel::arch::puts("\nGUI LOG OVERFLOW\n");
				return pos;
			}

			if self.putc(&bh, pos, colour, c) {
				pos = pos.next();
			}
		}
		pos
	}
	
	/// Writes a single codepoint to the display
	///
	/// Returns true if the character caused a cell change (i.e. it wasn't a combining character)
	fn putc(&self, bh: &crate::windows::WinBuf, pos: CharPos, colour: Colour, c: char) -> bool
	{
		// If the character was a combining AND it's not at the start of a line,
		// render atop the previous cell
		if c.is_combining() && pos.col() > 0 {
			self.render_char(bh, pos.prev(), colour, c);
			false
		}
		// Otherwise, wipe the cell and render into it
		else {
			self.clear_cell(bh, pos);
			self.render_char(bh, pos, colour, c);
			true
		}
	}
	
	// Low-level rendering
	/// Clear a character cell
	fn clear_cell(&self, bh: &crate::windows::WinBuf, pos: CharPos)
	{
		let Pos { x: bx, y: by } = pos.to_pixels();
		for row in 0 .. 16
		{
			let r = bh.scanline_rgn_mut(by as usize + row, bx as usize, 8); 
			for col in 0 .. 8
			{
				r[col] = 0;
			}
		}
	}
	/// Actually does the rendering
	//#[req_safe(taskswitch)]	//< Must be safe to call from within a spinlock
	fn render_char(&self, bh: &crate::windows::WinBuf, pos: CharPos, colour: Colour, cp: char)
	{
		if bh.dims().width() == 0 {
			return ;
		}

		let idx = unicode_to_cp437(cp);
		//log_trace!("KernelLog::render_char({:?}, {:?}, '{}') idx={}", pos, colour, cp, idx);
		
		let bitmap = &S_FONTDATA[idx as usize];
		
		// Actual render!
		let Pos { x: bx, y: by } = pos.to_pixels();
		for row in 0 .. 16
		{
			let byte = &bitmap[row as usize];
			let r = bh.scanline_rgn_mut(by as usize + row, bx as usize, 8); 
			for col in 0 .. 8
			{
				if (byte >> 7-col) & 1 != 0 {
					r[col] = colour.as_argb32();
				}
			}
		}
	}
}

impl CharPos
{
	fn col(&self) -> u32 { self.1 }
	fn next(self) -> CharPos { CharPos(self.0, self.1+1) }
	fn prev(self) -> CharPos { CharPos(self.0, self.1-1) }
	fn to_pixels(self) -> Pos {
		Pos::new( (self.1 * C_CELL_DIMS.w) as u32, (self.0 * C_CELL_DIMS.h) as u32 )
	}
}

impl LogWriter
{
	pub fn new(colour: Colour) -> LogWriter
	{
		S_KERNEL_LOG.get().reveal_new_line();
		// Subtract one, because the above has already added one
		let pos = CharPos(S_CURRENT_LINE.load(Ordering::Relaxed)-1,0);

		//let bh = S_BUFFER_HANDLE.get().unwrap();
		//log_trace!("bh = {:p}, pos={:?}, H={} {:?}", bh, pos, bh.dims().h / C_CELL_DIMS.h, pos.to_pixels());

		LogWriter {
			pos,
			colour,
			no_flush: false,
		}
	}
}
impl fmt::Write for LogWriter
{
	fn write_str(&mut self, s: &str) -> fmt::Result
	{
		self.pos = S_KERNEL_LOG.get().write_text(self.pos, self.colour, s);
		Ok( () )
	}
}
impl ::core::ops::Drop for LogWriter
{
	fn drop(&mut self)
	{
		//if !self.no_flush {
			S_KERNEL_LOG.get().flush();
		//}
	}
}

impl UnicodeCombining for char
{
	fn is_combining(&self) -> bool
	{
		match *self as u32
		{
		// Ranges from wikipedia:Combining_Character
		0x0300 ..= 0x036F => true,
		0x1AB0 ..= 0x1AFF => true,
		0x1DC0 ..= 0x1DFF => true,
		0x20D0 ..= 0x20FF => true,
		0xFE20 ..= 0xFE2F => true,
		_ => false,
		}
	}
}


mod tmp {
	use kernel::lib::mem::Arc;
	use core::sync::atomic::Ordering;
	pub struct AtomicArc<T>(::core::sync::atomic::AtomicPtr<T>);
	impl<T> AtomicArc<T>
	{
		pub const fn new() -> Self {
			AtomicArc(::core::sync::atomic::AtomicPtr::new(::core::ptr::null_mut()))
		}

		pub fn get(&self) -> Option<Arc<T>> {
			loop {
				let v = self.0.swap(usize::MAX as *mut T, Ordering::SeqCst);
				if v == usize::MAX as *mut T {
					continue ;
				}

				return if v == ::core::ptr::null_mut() {
					None
					}
					else {
						// SAFE: Pointer isn't the sentinel, and it's not NULL - and we've taken ownership of this handle
						let rv = unsafe { Arc::from_raw(v) };
						let stored = Arc::into_raw(rv.clone()) as *mut _;
						match self.0.compare_exchange(usize::MAX as *mut T, stored, Ordering::SeqCst, Ordering::SeqCst)
						{
						Ok(_marker) => {},
						// SAFE: Ownership of this handle has been returned
						Err(_old) => drop(unsafe { Arc::from_raw(stored) }),
						}
						Some(rv)
					};
			}
		}
		pub fn set(&self, v: Arc<T>) {
			let orig = self.0.swap(Arc::into_raw(v) as *mut  T, Ordering::SeqCst);
			if orig == ::core::ptr::null_mut() || orig == usize::MAX as *mut T {
			}
			else {
				// SAFE: We've taken ownership of this returned pointer
				drop(unsafe { Arc::from_raw(orig) });
			}
		}
	}
}

