
use super::Element;
use geom::Rect;

#[derive(PartialEq,Debug,Copy,Clone)]
enum Direction { Vertical, Horizontal }
impl Direction {
	fn is_vert(&self) -> bool { match self { &Direction::Vertical => true, &Direction::Horizontal => false } }

	fn get_rect(&self, ofs: u32, size: u32) -> Rect<::geom::Px> {
		if self.is_vert() {
			Rect::new(0, ofs, !0, size)
		} else {
			Rect::new(ofs, 0, size, !0)
		}
	}
}
#[derive(Copy,Clone)]
pub struct Size(u32);

pub struct Box<S: BoxEleSet> {
	direction: Direction,
	sizes: ::std::cell::RefCell<(u32,u32)>,
	elements: S,
}

pub struct BoxEle<E: Element> {
	size: Option<Size>,
	ele: E,
}
impl<E: Element> BoxEle<E>
{
	pub fn fixed(size: u32, ele: E) -> Self {
		BoxEle {
			size: Some(Size(size)),
			ele: ele,
		}
	}
	pub fn expand(ele: E) -> Self {
		BoxEle {
			size: None,
			ele: ele,
		}
	}

	pub fn inner(&self) -> &E {
		&self.ele
	}
}


impl<S: BoxEleSet> Box<S>
{
	fn new(dir: Direction, eles: S) -> Self {
		Box {
			direction: dir,
			sizes: Default::default(),
			elements: eles
		}
	}
	/// Create a vertically stacked box
	pub fn new_vert(eles: S) -> Self {
		Box::new(Direction::Vertical, eles)
	}
	/// Create a horizontally stacked box
	pub fn new_horiz(eles: S) -> Self {
		Box::new(Direction::Horizontal, eles)
	}

	pub fn inner(&self) -> &S {
		&self.elements
	}

	// returns (has_changed, expand_size)
	fn update_size(&self, cap: u32) -> (bool, u32) {
		let mut sizes = self.sizes.borrow_mut();
		if sizes.0 == cap {
			(false, sizes.1)
		}
		else {
			let expand = {
				let (fixed_total, num_expand) = self.elements.get_sizes();
				if fixed_total > cap {
					0
				}
				else if num_expand > 0 {
					(cap - fixed_total) / num_expand as u32
				}
				else {
					0
				}
				};
			*sizes = (cap, expand);
			(true, expand)
		}
	}
}
impl<S: BoxEleSet> super::Element for Box<S>
{
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut dyn crate::window::WindowTrait) -> bool {
		false
	}

	fn with_element_at_pos(&self, pos: ::geom::PxPos, dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool
	{
		// TODO: Use `dims`
		let (_cap, expand_size) = *self.sizes.borrow();
		self.elements.with_element_at_pos( pos, dims, f, expand_size, self.direction.is_vert() )
	}
	fn resize(&self, w: u32, h: u32)
	{
		let (is_dirty, expand_size) = self.update_size(if self.direction.is_vert() { h } else { w });

		let dir = self.direction;
		if is_dirty
		{
			self.elements.resize(w, h, expand_size, dir.is_vert());
		}
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		// 1. Determine sizes
		let (is_dirty, expand_size) = self.update_size(if self.direction.is_vert() { surface.height() } else { surface.width() });

		// 2. Render sub-surfaces
		let dir = self.direction;
		self.elements.render(surface, force || is_dirty, expand_size, |ofs,size| dir.get_rect(ofs, size));
	}
}

pub trait BoxEleSet
{
	fn count() -> usize;
	
	fn get_sizes(&self) -> (u32, usize);
	fn render<G>(&self, surface: ::surface::SurfaceView, force: bool, expand_size: u32, get_rect: G)
	where
		G: Fn(u32, u32)->Rect<::geom::Px>
		;
	fn resize(&self, w: u32, h: u32, expand_size: u32, is_vert: bool);
	fn with_element_at_pos(&self, pos: ::geom::PxPos, _dims: ::geom::PxDims, f: ::WithEleAtPosCb, exp: u32, is_vert: bool) -> bool;
}

macro_rules! impl_box_set_tuple {
	( $s:ident : $($n:ident = $v:expr),* ) => {
		impl<$($n: Element),*> BoxEleSet for ($(BoxEle<$n>,)*) {
			fn count() -> usize { $( ({ let _: $n; 1})+)* 0 }
			fn get_sizes(&$s) -> (u32, usize) {
				let mut fixed = 0;
				let mut expand = 0;
				$(
				if let Some(Size(v)) = $v.size {
					fixed += v;
				}
				else {
					expand += 1;
				}
				)*
				(fixed, expand)
			}
			fn resize(&$s, w: u32, h: u32, expand_size: u32, is_vert: bool)
			{
				$({
					let size = if let Some(Size(size)) = $v.size { size } else { expand_size };
					let s_w = if !is_vert { size } else { w };
					let s_h = if  is_vert { size } else { h };
					$v.ele.resize(s_w, s_h)
				})*
			}
			fn render<G>(&$s, surface: ::surface::SurfaceView, force: bool, expand_size: u32, get_rect: G)
			where
				G: Fn(u32, u32)->Rect<::geom::Px>
			{
				let mut ofs = 0;
				$({
					let size = if let Some(Size(size)) = $v.size { size } else { expand_size };
					let rect = get_rect(ofs, size);
					$v.ele.render(surface.slice(rect), force);
					ofs += size;
				})*
				let _ = ofs;
			}

			fn with_element_at_pos(&$s, p: ::geom::PxPos, dims: ::geom::PxDims, f: ::WithEleAtPosCb, exp: u32, is_vert: bool) -> bool {
				let pos = if is_vert { p.y } else { p.x };
				let pos = pos.0;
				let mut ofs = 0;
				$({
					let size = if let Some(Size(s)) = $v.size { s } else { exp };
					// If the cursor was before the right/bottom border of this element, it's within
					// - Works because of ordering
					if pos < ofs + size
					{
						return if is_vert {
								$v.ele.with_element_at_pos( p - ::geom::PxPos::new(0,ofs), ::geom::PxDims::new(dims.w.0, size), f )
							}
							else {
								$v.ele.with_element_at_pos( p - ::geom::PxPos::new(ofs,0), ::geom::PxDims::new(size, dims.h.0), f )
							};
					}
					else {
						ofs += size;
					}
				})*
				let _ = ofs;
				false
			}
		}
		};
}

impl_box_set_tuple!{ self : A = self.0 }
impl_box_set_tuple!{ self : A = self.0, B = self.1 }
impl_box_set_tuple!{ self : A = self.0, B = self.1, C = self.2 }
impl_box_set_tuple!{ self : A = self.0, B = self.1, C = self.2, D = self.3 }
impl_box_set_tuple!{ self : A = self.0, B = self.1, C = self.2, D = self.3, E = self.4 }

