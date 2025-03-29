
pub type PxPos = Pos<Px>;
pub type PxDims = Dims<Px>;

pub trait CoordType:
	Copy +
	::std::ops::Add<Output=Self> + ::std::ops::Sub<Output=Self> +
	//::std::ops::AddAssign + ::std::ops::SubAssign +
	::std::cmp::Ord +
	::std::fmt::Debug
{
	fn zero() -> Self;
	fn max_value() -> Self;
}

macro_rules! impl_prim_coord {
	($t:ident) => {
		#[derive(Copy,Clone,PartialOrd,PartialEq,Debug,Default)]
		pub struct $t(pub u32);
		impl From<u32> for $t { fn from(v: u32) -> $t { $t(v) } }
		impl CoordType for $t {
			fn max_value() -> $t { $t(!0) }
			fn zero() -> $t { $t(0) }
		}
		impl ::std::cmp::Eq for $t {}
		impl ::std::cmp::Ord for $t { fn cmp(&self, o: &$t) -> ::std::cmp::Ordering { self.partial_cmp(o).unwrap() } }
		impl ::std::ops::Add for $t { type Output = Self; fn add(self, v: Self) -> Self { $t(self.0.saturating_add(v.0)) } }
		impl ::std::ops::Sub for $t { type Output = Self; fn sub(self, v: Self) -> Self { $t(self.0.saturating_sub(v.0)) } }
		impl ::std::ops::Mul<u32> for $t { type Output = Self; fn mul(self, v: u32) -> Self { $t(self.0.checked_mul(v).unwrap_or(!0)) } }
		//impl ::std::ops::AddAssign for $t { fn add_assign(&mut self, v: Self) { self.0 += v.0 } }
		//impl ::std::ops::SubAssign for $t { fn sub_assign(&mut self, v: Self) { self.0 += v.0 } }
	}
}

impl_prim_coord!{ Px }
pub struct Unit(pub u32);
pub struct Mm(pub u32);

#[derive(Copy,Clone,Default)]
pub struct Rect<T: CoordType>
{
	pub x: T,
	pub y: T,
	pub w: T,
	pub h: T,
}
impl<T: CoordType> ::std::fmt::Debug for Rect<T>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		write!(f, "Rect{{ ({:?},{:?}) {:?}x{:?} }}", self.x, self.y, self.w, self.h)
	}
}

#[derive(Copy,Clone,Default,PartialEq,Eq)]
pub struct Pos<T: CoordType> {
	pub x: T,
	pub y: T,
}
impl<T: CoordType> ::std::fmt::Debug for Pos<T>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		write!(f, "Pos({:?},{:?})", self.x, self.y)
	}
}

#[derive(Copy,Clone,Default,PartialEq,Eq)]
pub struct Dims<T: CoordType> {
	pub w: T,
	pub h: T,
}
impl<T: CoordType> ::std::fmt::Debug for Dims<T>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		write!(f, "Dims({:?}x={:?})", self.w, self.h)
	}
}

impl<T: CoordType> Rect<T>
{
	pub fn new<U: Into<T>>(x: U, y: U, w: U, h: U) -> Rect<T> {
		Rect {
			x: x.into(),
			y: y.into(),
			w: w.into(),
			h: h.into(),
		}
	}
	pub fn new_pts<U: Into<T>>(x: U, y: U, x2: U, y2: U) -> Rect<T> {
		let x = x.into();
		let y = y.into();
		let x2 = x2.into();
		let y2 = y2.into();
		Rect {
			x: x,
			y: y,
			w: if x2 > x { x2 - x } else { T::zero() },
			h: if y2 > y { y2 - y } else { T::zero() },
		}
	}
	pub fn new_full() -> Rect<T> {
		Rect {
			x: T::zero(),
			y: T::zero(),
			w: T::max_value(),
			h: T::max_value(),
		}
	}

	pub fn is_empty(&self) -> bool {
		self.w == T::zero() || self.h == T::zero()
	}

	pub fn dims(&self) -> Dims<T> {
		Dims::new(self.w, self.h)
	}
	pub fn with_dims(&self, d: Dims<T>) -> Self {
		Self {
			x: self.x,
			y: self.y,
			w: d.w,
			h: d.h,
		}
	}

	pub fn width(&self) -> T { self.w }
	pub fn height(&self) -> T { self.h }

	pub fn x(&self) -> T { self.x }
	pub fn y(&self) -> T { self.y }
	pub fn x2(&self) -> T { self.x + self.w }
	pub fn y2(&self) -> T { self.y + self.h }

	pub fn top_left(&self) -> Pos<T> {
		Pos { x: self.x, y: self.y }
	}
	pub fn contains(&self, pos: Pos<T>) -> bool {
		self.x() <= pos.x && pos.x < self.x2()
			&& self.y() <= pos.y && pos.y < self.y2()
	}

	/// Get the trivial union of two rectangles (i.e. the smallest rect containing both)
	pub fn union(&self, other: &Rect<T>) -> Rect<T> {
		let x = ::std::cmp::min(self.x, other.x);
		let y = ::std::cmp::min(self.y, other.y);
		let x2 = ::std::cmp::max(self.x2(), other.x2());
		let y2 = ::std::cmp::max(self.y2(), other.y2());
		Rect::new_pts(x, y, x2, y2)
	}

	/// Get the intersection of two rectangles
	pub fn intersect(&self, other: &Rect<T>) -> Rect<T> {
		let x = ::std::cmp::max(self.x, other.x);
		let y = ::std::cmp::max(self.y, other.y);
		let ox = ::std::cmp::min(self.x2(), other.x2());
		let oy = ::std::cmp::min(self.y2(), other.y2());

		let rv = Rect::new_pts(x, y, ox, oy);
		//kernel_log!("Rect::intersect {:?} int {:?} = {:?}", self, other, rv);
		rv
	}

	/// Obtain a new rect offset from this by x,y
	pub fn offset(&self, x: T, y: T) -> Rect<T> {
		Rect {
			x: self.x + x,
			y: self.y + y,
			w: self.w,
			h: self.h,
		}
	}

	/// Get the absolute rect from a relative rect.
	pub fn relative(&self, other: &Rect<T>) -> Rect<T> {
		let (x, ox) = if other.x < self.w {
				( self.x + other.x, ::std::cmp::min(self.x2(), self.x + other.x2()) )
			}
			else {
				( self.x + self.w, T::zero() )
			};
		let (y, oy) = if other.y < self.h {
				( self.y + other.y, ::std::cmp::min(self.y2(), self.y + other.y2()) )
			}
			else {
				( self.y + self.h, T::zero() )
			};

		let rv = Rect::new_pts(x, y, ox, oy);
		//kernel_log!("Rect::relative {:?} int {:?} = {:?}", self, other, rv);
		rv
	}
}

impl<T: CoordType> Pos<T>
{
	pub fn new<U: Into<T>>(x: U, y: U) -> Pos<T> {
		Pos {
			x: x.into(),
			y: y.into(),
			}
	}
}
impl<T: CoordType> ::std::ops::Sub for Pos<T>
{
	type Output = Self;
	fn sub(self, other: Self) -> Self {
		Pos {
			x: self.x - other.x,
			y: self.y - other.y,
			}
	}
}
impl<T: CoordType> Dims<T>
{
	pub fn new<U: Into<T>>(w: U, h: U) -> Dims<T> {
		Dims {
			w: w.into(),
			h: h.into(),
		}
	}

	pub fn bottomright(&self) -> Pos<T> {
		Pos {
			x: self.w,
			y: self.h,
			}
	}
}
impl<T: CoordType> ::std::ops::Sub for Dims<T>
{
	type Output = Self;
	fn sub(self, other: Self) -> Self {
		Dims {
			w: self.w - other.w,
			h: self.h - other.h,
			}
	}
}
