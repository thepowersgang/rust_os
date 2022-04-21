// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video/mod.rs
/// Geometry types
#[allow(unused_imports)]
use crate::prelude::*;

#[derive(Copy,Clone,PartialEq,Default)]
pub struct Pos
{
	pub x: u32,
	pub y: u32,
}
#[derive(Copy,Clone,PartialEq,Default)]
pub struct Dims
{
	pub w: u32,
	pub h: u32,
}
#[derive(Copy,Clone,PartialEq,Default)]
pub struct Rect
{
	pub pos: Pos,
	pub dims: Dims,
}


impl Pos
{
	/// Construct a new `Pos`
	pub const fn new(x: u32, y: u32) -> Pos {
		Pos { x: x, y: y }
	}

	pub fn dist_sq(&self, other: &Pos) -> u64 {
		let dx = (self.x - other.x) as u64;
		let dy = (self.y - other.y) as u64;
		dx*dx + dy*dy
	}

	pub fn offset(&self, dx: i32, dy: i32) -> Pos {
		Pos {
			x: (self.x as i32 + dx) as u32,
			y: (self.y as i32 + dy) as u32,
			}
	}
}
impl ::core::ops::Sub<Pos> for Pos {
	type Output = Pos;
	fn sub(self, other: Pos) -> Pos {
		Pos {
			x: self.x - other.x,
			y: self.y - other.y,
			}
	}
}

impl Dims
{
	/// Construct a new `Dims` struct
	pub const fn new(w: u32, h: u32) -> Dims {
		Dims { w: w, h: h }
	}

	/// Return the height
	pub fn height(&self) -> u32 { self.h }
	/// Return the width
	pub fn width(&self) -> u32 { self.w }

	pub fn min_of(&self, other: &Dims) -> Dims {
		Dims {
			w: ::core::cmp::min(self.w, other.w),
			h: ::core::cmp::min(self.h, other.h),
		}
	}
}

impl Rect
{
	/// Construct a new `Rect`
	pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
		Rect {
			pos: Pos { x: x, y: y },
			dims: Dims::new(w,h),
		}
	}
	/// Construct a new rect from a position and dimensions
	pub const fn new_pd(pos: Pos, dims: Dims) -> Rect {
		Rect { pos: pos, dims: dims }
	}
	
	/// Returns true if this rect fits within the provided area
	pub fn within(&self, w: u32, h: u32) -> bool {
		self.x() < w && self.y() < h
		&& self.w() <= w && self.h() <= h
		&& self.x() + self.w() <= w && self.y() + self.h() <= h
	}
	
	pub fn pos(&self) -> Pos { self.pos }
	pub fn dims(&self) -> Dims { self.dims }
	
	pub fn x(&self) -> u32 { self.pos.x }
	pub fn y(&self) -> u32 { self.pos.y }
	pub fn w(&self) -> u32 { self.dims.w }
	pub fn h(&self) -> u32 { self.dims.h }
	
	pub fn top(&self) -> u32 { self.y() }
	pub fn left(&self) -> u32 { self.x() }
	pub fn right(&self) -> u32 { self.x() + self.w() }
	pub fn bottom(&self) -> u32 { self.y() + self.h() }
	
	/// Returns the top-left point
	pub fn tl(&self) -> Pos { self.pos }
	/// Returns the bottom-right point
	pub fn br(&self) -> Pos { Pos::new( self.x() + self.w(), self.y() + self.h() ) }
	/// Returns the "inner" bottom-right point (pointing to within the rect)
	pub fn br_inner(&self) -> Pos { Pos::new( self.x() + self.w() - 1, self.y() + self.h() - 1 ) }


	/// Obtain the closest Pos in this Rect
	pub fn clamp_pos(&self, pos: Pos) -> Pos {
		let x = ::core::cmp::min( ::core::cmp::max(pos.x, self.left()), self.right() - 1  );
		let y = ::core::cmp::min( ::core::cmp::max(pos.y, self.top() ), self.bottom() - 1 );
		Pos::new(x, y)
	}

	
	/// Returns true if this rect contains the provided point
	pub fn contains(&self, pt: &Pos) -> bool {
		(self.left() <= pt.x && pt.x < self.right()) && (self.top() <= pt.y && pt.y < self.bottom())
	}
	/// Returns true if this rect wholly contains the passed rect
	pub fn contains_rect(&self, r: &Rect) -> bool {
		if ! self.contains( &r.tl() ) {
			false
		}
		else if r.w() == 0 || r.h() == 0 {
			true
		}
		else if self.contains( &r.br_inner() ) {
			true
		}
		else {
			false
		}
	}
	
	/// Returns the intersection of this rect and another (or None if no overlap)
	pub fn intersect(&self, other: &Rect) -> Option<Rect> {
		// Intersection:
		//  MAX(X1) MAX(Y1)  MIN(X2) MIN(Y2)
		let max_x1 = ::core::cmp::max( self.left(), other.left() );
		let max_y1 = ::core::cmp::max( self.top() , other.top() );
		let min_x2 = ::core::cmp::min( self.right() , other.right() );
		let min_y2 = ::core::cmp::min( self.bottom(), other.bottom() );
		
		//log_trace!("Rect::intersect({} with {}) = ({},{}) ({},{})", self, other, max_x1, max_y1, min_x2, min_y2);
		
		if max_x1 < min_x2 && max_y1 < min_y2 {
			Some( Rect {
				pos: Pos { x: max_x1, y: max_y1 },
				dims: Dims::new(min_x2 - max_x1, min_y2 - max_y1)
				} )
		}
		else {
			None
		}
	}
	
	/// Iterates areas in `self` that don't intersect with `other`
	pub fn not_intersect<'a>(&'a self, other: &'a Rect) -> NotIntersect<'a> {
		NotIntersect {
			left: self,
			right: self.intersect(other).unwrap_or(Rect::new(0,0,0,0)),
			idx: 0,
		}
	}
	
	/// Returns the loose union of two rects (i.e. the smallest rect that contains both)
	pub fn union(&self, other: &Rect) -> Rect
	{
		let new_tl = Pos {
			x: ::core::cmp::min(self.left(), other.left()), 
			y: ::core::cmp::min(self.top(),  other.top() )
			};
		let new_br = Pos {
			x: ::core::cmp::max(self.right(),  other.right() ), 
			y: ::core::cmp::max(self.bottom(), other.bottom())
			};
		Rect {
			pos: new_tl,
			dims: Dims::new( new_br.x - new_tl.x, new_br.y - new_tl.y ),
		}
	}
	
	/// Iterate over intersections of two slices of `Rect`
	pub fn list_intersect<'a>(list1: &'a [Rect], list2: &'a [Rect]) -> RectListIntersect<'a> {
		RectListIntersect {
			list1: list1,
			list2: list2,
			idx1: 0,
			idx2: 0,
		}
	}
}

/// Iterator over the negative intersection of two `Rect`s, yields up to 4 rects
pub struct NotIntersect<'a>
{
	left: &'a Rect,
	right: Rect,
	idx: usize,
}
impl<'a> Iterator for NotIntersect<'a>
{
	type Item = Rect;
	fn next(&mut self) -> Option<Rect>
	{
		if self.right.w() == 0 {
			return None;
		}
		// Max of four possible rects can be generated.
		// NOTE: The following algos assume that `self.right` is a strict subset of `self.left`
		while self.idx < 4
		{
			let cur = self.idx;
			self.idx += 1;
			match cur
			{
			// - Area above the intersection (long)
			0 => if self.left.top() < self.right.top() {
					// Same TL and W as left, H = delta top
					return Some( Rect::new_pd(
						self.left.pos(),
						Dims::new(self.left.w(), self.right.top()-self.left.top())
						) );
				},
			// - Left of the intersection (short)
			1 => if self.left.left() < self.right.left() {
					// (Left Left, Right Top) W = delta left, H = right height
					return Some( Rect::new(
						self.left.x(), self.right.y(),
						self.right.left() - self.left.left(), self.right.h()
						) );
				},
			// - Right of the intersection (short)
			2 => if self.left.right() > self.right.right() {
					// (Right Left, Right Top) W = delta left, H = right height
					return Some( Rect::new(
						self.right.right(), self.right.top(),
						self.left.right() - self.right.right(), self.right.h()
						) );
				},
			// - Area below the intersection (long)
			3 => if self.left.bottom() > self.right.bottom() {
					return Some( Rect::new(
						self.left.left(), self.right.bottom(),
						self.left.w(), self.left.bottom() - self.right.bottom()
						) );
				},
			_ => unreachable!(),
			}
		}
		None
	}
}

/// Iterator over the intersections of two `Rect` slices
pub struct RectListIntersect<'a>
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
				self.idx1 += 1;
				self.idx2 = 0;
			}
			else {
				let rv = self.list1[self.idx1].intersect( &self.list2[self.idx2] );
				self.idx2 += 1;
				if rv.is_some() {
					return rv;
				}
			}
		}
		None
	}
}

impl_fmt! {
	Debug(self, f) for Pos { write!(f, "({},{})", self.x, self.y) }
	Debug(self, f) for Dims { write!(f, "{}x{}", self.w, self.h) }
	Debug(self, f) for Rect { write!(f, "({},{} + {}x{})", self.x(), self.y(), self.w(), self.h()) }
	Display(self, f) for Rect { write!(f, "({},{} + {}x{})", self.x(), self.y(), self.w(), self.h()) }
}
