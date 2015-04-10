// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/mod.rs
// - Kernel compositor core
//! Kernel-mode side of the GUI
//! Provides input routing and window management (i.e. exposing buffers to userland)
/*!
Design Notes
===
- Group windows into "screens" based on the owning session
- When a screen is hidden, a signalling message is sent to the controlling program (similar to the session leader in POSIX)
 - This allows the leader to switch to a lock screen
- All windows are backed by a framebuffer in this code
 - Kernel log is provided by a builtin text renderer
*/
use _common::*;
module_define!{GUI, [Video], init}

/// Initialise the GUI
fn init()
{
	// - Enumerate display devices
	//::metadevs::video::register_enumerate( enum_displays );
	// - Create kernel logging screen+window
	windows::init();
	kernel_log::init();
}

fn enum_displays(was_added: bool, index: ::metadevs::video::FramebufferRef)
{
	if !was_added {
		unimplemented!();
	}
	else {
		// Add this output to the multidisplay grid
	}
}

/// General window handling code
mod windows;
/// Kernel log display
mod kernel_log;

/// Dimensions : Width/Height
#[derive(Copy,Clone,Debug,Default)]
struct Dims(u32,u32);	// W, H
/// Position : X/Y
#[derive(Copy,Clone,Debug,Default)]
struct Pos(i32,i32);
/// A generic rectangle
#[derive(Copy,Clone,Debug)]
struct Rect(Pos,Dims);
/// Pixel colour
#[derive(Copy,Clone)]
struct Colour(u32);

impl Pos
{
	pub fn x(&self) -> i32 { self.0 }
	pub fn y(&self) -> i32 { self.1 }
}
impl Dims
{
	pub fn width (&self) -> u32 { self.0 }
	pub fn height(&self) -> u32 { self.1 }
}

impl Rect
{
	pub fn max() -> Rect { Rect(Pos(0,0), Dims(u32::max_value(), u32::max_value())) }

	pub fn pos(&self) -> Pos { self.0 }
	pub fn epos(&self) -> Pos {
		Pos( self.pos().0 + self.dim().0 as i32, self.pos().1 + self.dim().1 as i32 )
	}
	pub fn dim(&self) -> Dims { self.1 }
	
	pub fn top(&self) -> i32 { self.0 .1 }
	pub fn left(&self) -> i32 { self.0 .0 }
	pub fn right(&self) -> i32 { self.0 .0 + self.dim().width() as i32 }
	pub fn bottom(&self) -> i32 { self.0 .1 + self.dim().height() as i32 }
	
	pub fn intersect(&self, other: &Rect) -> Option<Rect> {
		// Intersection:
		//  MAX(X1) MAX(Y1)  MIN(X2) MIN(Y2)
		let max_x1 = ::core::cmp::max( self.pos().0, other.pos().0 );
		let max_y1 = ::core::cmp::max( self.pos().1, other.pos().1 );
		let min_x2 = ::core::cmp::max( self.epos().0, other.epos().0 );
		let min_y2 = ::core::cmp::max( self.epos().1, other.epos().1 );
		
		if max_x1 < min_x2 && max_y1 < min_y2 {
			Some( Rect(
				Pos(max_x1, max_y1),
				Dims((min_x2 - max_x1) as u32, (min_y2 - max_y1) as u32)
				) )
		}
		else {
			None
		}
	}
	
	pub fn list_intersect<'a>(list1: &'a [Rect], list2: &'a [Rect]) -> RectListIntersect<'a> {
		RectListIntersect {
			list1: list1,
			list2: list2,
			idx1: 0,
			idx2: 0,
		}
	}
}

struct RectListIntersect<'a>
{
	list1: &'a [Rect],
	list2: &'a [Rect],
	idx1: usize,
	idx2: usize,
}
impl<'a> Iterator for RectListIntersect<'a>
{
	type Item = Rect;
	fn next(&mut self) -> Option<Rect>
	{
		// Iterate list1, iterate list2
		while self.idx1 < self.list1.len()
		{
			if self.idx2 == self.list2.len() {
				self.idx2 = 0;
				self.idx1 += 1;
			}
			else {
				let rv = self.list1[self.idx1].intersect( &self.list2[self.idx2] );
				if rv.is_some() {
					return rv;
				}
			}
		}
		None
	}
}

impl_fmt!{
	Debug(self, f) for Colour { write!(f, "Colour({:06x})", self.0) }
}
impl Colour
{
	pub fn def_black() -> Colour { Colour(0x00_00_00) }
	pub fn def_white() -> Colour { Colour(0xFF_FF_FF) }
	
	pub fn def_yellow() -> Colour { Colour(0xFF_FF_00) }
	
	pub fn as_argb32(&self) -> u32 { self.0 }
}

