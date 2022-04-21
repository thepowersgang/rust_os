// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video/mod.rs
///! Video (Display) management
use crate::prelude::*;
use crate::sync::mutex::Mutex;
use crate::lib::sparse_vec::SparseVec;

pub use self::geom::{Pos,Dims,Rect};

module_define!{Video, [], init}

/// Pre-driver graphics support (using a bootloader provided video mode)
pub mod bootvideo;

/// Kernel-provided bitmap font
pub mod kernel_font;

/// Geometry types (Pos, Dims, Rect)
mod geom;

/// Mouse cursor handle and rendering
pub mod cursor;

pub use self::cursor::CursorHandle;

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
	fn as_any(&self) -> &dyn Any;
	fn activate(&mut self);
	
	fn get_size(&self) -> Dims;
	fn set_size(&mut self, newsize: Dims) -> bool;
	
	fn blit_inner(&mut self, dst: Rect, src: Rect);
	fn blit_ext(&mut self, dst: Rect, src: Rect, srf: &dyn Framebuffer) -> bool;
	// NOTE: Takes a buffer with stride to allow blitting from a source rect
	fn blit_buf(&mut self, dst: Rect, buf: StrideBuf<'_,u32>);
	fn fill(&mut self, dst: Rect, colour: u32);
	
	fn move_cursor(&mut self, p: Option<Pos>);
	//fn set_cursor(&mut self, size: Dims, data: &[u32]);
	// TODO: Handle 3D units
}

/// Wrapper around a buffer that includes a stride (width between contiguious segments)
pub struct StrideBuf<'a, T: 'a>
{
	buf: &'a [T],
	stride: usize,
}
impl<'a, T: 'a> StrideBuf<'a, T>
{
	pub fn new(buf: &[T], stride: usize) -> StrideBuf<T> {
		StrideBuf {
			buf, stride
			}
	}

	pub fn stride(&self) -> usize {
		self.stride
	}
	pub fn tail_width(&self) -> usize {
		let rem = self.buf.len() % self.stride;
		if rem == 0 {
			self.stride
		}
		else {
			rem
		}
	}
	pub fn count(&self) -> usize {
		(self.buf.len() + self.stride - 1) / self.stride
	}

	/// Checks that the size of the last chunk is divisible by the given divisor
	pub fn is_round(&self, divisor: usize) -> bool
	{
		self.stride >= divisor && self.buf.len() % self.stride % divisor == 0
	}
	/// Return chunks of the given width
	pub fn chunks(&self, w: usize) -> impl ::core::iter::ExactSizeIterator<Item=&'a [T]>
	{
		assert!(w <= self.stride);
		self.buf.chunks(self.stride).map(move |v| if v.len() < w { v } else { &v[..w] })
	}

	/// Take a sub-rect with the given x/y as the left/top
	pub fn offset(&self, x: usize, y: usize) -> StrideBuf<'a, T>
	{
		let ofs = x + y * self.stride;
		StrideBuf {
			buf: &self.buf[ofs..],
			stride: self.stride,
			}
	}
	pub fn clip(&self, w: usize, h: usize) -> StrideBuf<'a, T>
	{
		assert!(w <= self.stride);
		assert!(h >= 1);
		// The last line has to be `w` lines long
		let new_len = (h-1) * self.stride + w;
		StrideBuf {
			buf: &self.buf[..new_len],
			stride: self.stride,
			}
	}
}
impl<'a, T: 'a> ::core::fmt::Pointer for StrideBuf<'a, T>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "StrideBuf({:p} w={},h={}, s={})", self.buf.as_ptr(), self.tail_width(), self.count(), self.stride())
	}
}

struct DisplaySurface
{
	region: Rect,
	fb: Box<dyn Framebuffer>,
}

/// Sparse list of registered display devices
static S_DISPLAY_SURFACES: Mutex<SparseVec<DisplaySurface>> = Mutex::new(SparseVec::new());
/// Boot video mode
static S_BOOT_MODE: Mutex<Option<bootvideo::VideoMode>> = Mutex::new(None);
/// Function called when display geometry changes
static S_GEOM_UPDATE_SIGNAL: Mutex<Option<fn(new_total: Rect)>> = Mutex::new(None);

fn init()
{
	if let Some(mode) = S_BOOT_MODE.lock().as_ref()
	{
		log_notice!("Using boot video mode {:?}", mode);
		let fb = box bootvideo::Framebuffer::new(*mode) as Box<dyn Framebuffer>;
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


// A picture of a sad ferris the crab
// NOTE: Commented out, as uncompressed 32bpp is too large to fit in the image
include!{"../../../../Graphics/.output/shared/panic.rs"}
pub fn set_panic(file: &str, line: usize, message: &::core::fmt::Arguments)
{
	use core::sync::atomic::{AtomicBool, Ordering};
	static LOOP_PREVENT: AtomicBool = AtomicBool::new(false);
	if LOOP_PREVENT.swap(true, Ordering::Relaxed) {
		return ;
	}
	const PANIC_COLOUR: u32 = 0x01346B;
	const PANIC_TEXT_COLOUR: u32 = 0xFFFFFF;
	static mut PANIC_IMG_ROW_BUF: [u32; PANIC_IMAGE_DIMS.0 as usize] = [0; PANIC_IMAGE_DIMS.0 as usize];

	// SAFE: `LOOP_PREVENT` prevents this code from running over itself
	let row_buf = unsafe { &mut PANIC_IMG_ROW_BUF };

	for surf in S_DISPLAY_SURFACES.lock().iter_mut()
	{
		use core::fmt::Write;
		let dims = surf.fb.get_size();
		// 1. Fill
		surf.fb.fill(Rect::new_pd(Pos::new(0,0), dims), PANIC_COLOUR);
		// 2. Draw a sad ferris
		if dims.w >= PANIC_IMAGE_DIMS.0 && dims.h >= PANIC_IMAGE_DIMS.1 {
			let p = Pos::new(
				(dims.w - PANIC_IMAGE_DIMS.0) / 2,
				(dims.h - PANIC_IMAGE_DIMS.1) / 2,
				);
			for (y,row) in PANIC_IMAGE_DATA.iter().enumerate() {
				row.decompress(row_buf);
				let p = p.offset(0,y as i32);
				let r = Rect::new_pd(p, Dims::new(PANIC_IMAGE_DIMS.0, 1));
				surf.fb.blit_buf(r, StrideBuf::new(row_buf, PANIC_IMAGE_DIMS.0 as usize));
			}
		}
		// 3. Render message to top-left
		let _ = write!(&mut PanicWriter::new(&mut *surf.fb, 0, 0 , dims.w as u16), "Panic at {}:{}", file, line);
		let _ = write!(&mut PanicWriter::new(&mut *surf.fb, 0, 16, dims.w as u16), "- {}", message);
	}

	return ;

	struct PanicWriter<'a> {
		font: kernel_font::KernelFont,
		out: PanicWriterOut<'a>
	}
	impl<'a> ::core::fmt::Write for PanicWriter<'a> {
		fn write_char(&mut self, c: char) -> ::core::fmt::Result {
			let out = &mut self.out;
			self.font.putc(PANIC_TEXT_COLOUR, c, |buf| out.putc(buf));
			Ok( () )
		}
		fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
			for c in s.chars() {
				self.write_char(c)?;
			}
			Ok( () )
		}
	}
	impl<'a> PanicWriter<'a> {
		fn new<'b>(fb: &'b mut dyn Framebuffer, x: u16, y: u16, w: u16) -> PanicWriter<'b> {
			PanicWriter {
				font: kernel_font::KernelFont::new(PANIC_COLOUR),
				out: PanicWriterOut {
					fb: fb,
					x: x, y: y, w: w,
					},
				}
		}
	}
	impl<'a> ::core::ops::Drop for PanicWriter<'a> {
		fn drop(&mut self) {
			self.out.putc( self.font.get_buf() );
		}
	}
	struct PanicWriterOut<'a> {
		fb: &'a mut dyn Framebuffer,
		x: u16, y: u16, w: u16,
	}
	impl<'a> PanicWriterOut<'a>
	{
		fn putc(&mut self, data: &[u32; 8*16]) {
			self.fb.blit_buf(
				Rect::new(self.x as u32, self.y as u32,  8, 16),
				StrideBuf::new(data, 8)
				);
			self.x += 8;
			if self.x == self.w {
				self.y += 16;
				self.x = 0;
			}
		}
	}
}

/// Set the boot video mode.
///
/// NOTE: Must be called before this module is initialised to have any effect
pub fn set_boot_mode(mode: bootvideo::VideoMode)
{
	let mut lh = S_BOOT_MODE.lock();
	assert!(lh.is_none(), "Boot video mode set multiple times");
	*lh = Some(mode);
}

/// Register a function to be called when the display dimensions change
pub fn register_geom_update(fcn: fn(new_total: Rect))
{
	let mut lh = S_GEOM_UPDATE_SIGNAL.lock();
	assert!(lh.is_none(), "register_geom_update called multiple times (prev {:p}, new {:p})", lh.as_ref().unwrap(), &fcn);
	*lh = Some(fcn);
}

fn signal_geom_update(surfs: crate::sync::mutex::HeldMutex<SparseVec<DisplaySurface>>)
{
	// API Requirements
	// - New surface added (with location)
	// - Old surface removed
	// - Surface relocated (as part of removal/sorting/editing)
	// > Could just have a generic "things changed" call and let the GUI/user poll request the new state
	if let Some(fcn) = *S_GEOM_UPDATE_SIGNAL.lock()
	{
		let total_area = surfs.iter().map(|x| x.region).fold(Rect::new(0,0,0,0), |a,b| a.union(&b));
		log_trace!("signal_geom_update: total_area={:?}", total_area);
		drop( surfs );
		fcn( total_area );
	}
}

/// Add an output to the display manager
pub fn add_output(output: Box<dyn Framebuffer>) -> FramebufferRegistration
{
	let dims = output.get_size();
	// Detect if the boot mode is still present, and clear if it is
	if S_BOOT_MODE.lock().take().is_some()
	{
		// - Remove boot video framebuffer
		log_notice!("Alternative display driver loaded, dropping boot video");
		S_DISPLAY_SURFACES.lock().remove(0);
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
	
	signal_geom_update(lh);
	
	log_debug!("Registering framebuffer #{}", idx);
	FramebufferRegistration {
		reg_id: idx
	}
}


fn with_display_at_pos<R, F>(pos: Pos, fcn: F) -> Option<R>
where
	F: FnOnce(&mut DisplaySurface) -> R
{
	for surf in S_DISPLAY_SURFACES.lock().iter_mut()
	{
		if surf.region.contains(&pos)
		{
			return Some(fcn(surf));
		}
	}
	None
}

fn get_closest_visible_pos(pos: Pos) -> Pos
{
	let (mut dist, mut cpos) = (!0, Pos::new(0, 0));

	for surf in S_DISPLAY_SURFACES.lock().iter_mut()
	{
		if surf.region.contains(&pos) {
			return pos;
		}

		let new_pos = surf.region.clamp_pos(pos);
		let new_dist = new_pos.dist_sq(&pos);
		if new_dist < dist {
			dist = new_dist;
			cpos = new_pos;
		}
	}

	cpos
}

/// Returns the display region that contains the given point
pub fn get_display_for_pos(pos: Pos) -> Result<Rect,Rect>
{
	with_display_at_pos(pos, |s| s.region)
	// If the position is outside of any displays, return the closest display to it
		.ok_or_else( || with_display_at_pos(get_closest_visible_pos(pos), |s| s.region).unwrap_or(Rect::default()) )
}

/// Write part of a single scanline to the screen
///
/// Unsafe because it (eventually) will be able to cause multiple writers
// TODO: Why is this unsafe?
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
			let buf = StrideBuf::new(&data[ofs_l as usize .. ofs_r as usize], (ofs_r - ofs_l) as usize);
			surf.fb.blit_buf(sub_rect, buf);
		}
	}
}
pub /*unsafe*/ fn write_buf(pos: Pos, data: StrideBuf<'_, u32>)
{
	let rect = Rect { pos: pos, dims: Dims::new(data.tail_width() as u32, data.count() as u32) };
	// 1. Locate surface
	for surf in S_DISPLAY_SURFACES.lock().iter_mut()
	{
		if let Some(sub_rect) = surf.region.intersect(&rect)
		{
			let ofs_l = (sub_rect.left() - rect.left()) as usize;
			let ofs_t = (sub_rect.top() - rect.top()) as usize;
			// Get sub-slice
			let buf = data.offset(ofs_l, ofs_t).clip(sub_rect.w() as usize, sub_rect.h() as usize);
			// 2. Blit to it
			surf.fb.blit_buf(sub_rect, buf);
		}
	}
}

impl ::core::ops::Drop for FramebufferRegistration
{
	fn drop(&mut self)
	{
		let mut lh = S_DISPLAY_SURFACES.lock();
		lh.remove(self.reg_id);
		signal_geom_update(lh);
	}
}


// vim: ft=rust
