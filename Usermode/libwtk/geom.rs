

pub trait CoordType: Copy + ::std::ops::Add<Output=Self> + ::std::ops::Sub<Output=Self> + ::std::cmp::Ord + ::std::num::Zero + ::std::fmt::Debug
{
	fn max_value() -> Self;
}

macro_rules! impl_prim_coord {
	($t:ident) => {
		#[derive(Copy,Clone,PartialOrd,PartialEq,Debug)]
		pub struct $t(pub u32);
		impl From<u32> for $t { fn from(v: u32) -> $t { $t(v) } }
		impl CoordType for $t {
			fn max_value() -> $t { $t(!0) }
		}
		impl ::std::cmp::Eq for $t {}
		impl ::std::cmp::Ord for $t { fn cmp(&self, o: &$t) -> ::std::cmp::Ordering { self.partial_cmp(o).unwrap() } }
		impl ::std::ops::Add for $t { type Output = Self; fn add(self, v: Self) -> Self { $t(self.0.saturating_add(v.0)) } }
		impl ::std::ops::Sub for $t { type Output = Self; fn sub(self, v: Self) -> Self { $t(self.0.saturating_sub(v.0)) } }
		impl ::std::num::Zero for $t { fn zero() -> $t { $t(0) } }
	}
}

impl_prim_coord!{ Px }
pub struct Unit(u32);
pub struct Mm(u32);

#[derive(Copy,Clone)]
pub struct Rect<T: CoordType>
{
	x: T,
	y: T,
	w: T,
	h: T,
}
impl<T: CoordType> ::std::fmt::Debug for Rect<T>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		write!(f, "Rect{{ ({:?},{:?}) {:?}x{:?} }}", self.x, self.y, self.w, self.h)
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

	pub fn width(&self) -> T { self.w }
	pub fn height(&self) -> T { self.h }

	pub fn x(&self) -> T { self.x }
	pub fn y(&self) -> T { self.y }
	pub fn x2(&self) -> T { self.x + self.w }
	pub fn y2(&self) -> T { self.y + self.h }

	pub fn intersect(&self, other: &Rect<T>) -> Rect<T> {
		let x = ::std::cmp::max(self.x, other.x);
		let y = ::std::cmp::max(self.y, other.y);
		let ox = ::std::cmp::min(self.x2(), other.x2());
		let oy = ::std::cmp::min(self.y2(), other.y2());

		let rv = Rect::new_pts(x, y, ox, oy);
		//kernel_log!("Rect::intersect {:?} int {:?} = {:?}", self, other, rv);
		rv
	}

	pub fn offset(&self, x: T, y: T) -> Rect<T> {
		Rect {
			x: self.x + x,
			y: self.y + y,
			w: self.w,
			h: self.h,
		}
	}

	pub fn relative(&self, other: &Rect<T>) -> Rect<T> {
		let x = self.x + other.x;
		let y = self.y + other.y;
		let ox = ::std::cmp::min(self.x2(), self.x + other.x2());
		let oy = ::std::cmp::min(self.y2(), self.y + other.y2());

		let rv = Rect::new_pts(x, y, ox, oy);
		//kernel_log!("Rect::relative {:?} int {:?} = {:?}", self, other, rv);
		rv
	}
}
