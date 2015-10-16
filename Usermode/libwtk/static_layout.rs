
use super::Element;
use geom::Rect;

#[derive(PartialEq,Debug)]
enum Direction { Vertical, Horizontal }
impl Direction {
	fn is_vert(&self) -> bool { match self { &Direction::Vertical => true, &Direction::Horizontal => false } }
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
	pub fn fixed(size: u32, ele: E) -> BoxEle<E> {
		BoxEle {
			size: Some(Size(size)),
			ele: ele,
		}
	}
	pub fn expand(ele: E) -> BoxEle<E> {
		BoxEle {
			size: None,
			ele: ele,
		}
	}
}


impl<S: BoxEleSet> Box<S>
{
	fn new(dir: Direction, eles: S) -> Box<S> {
		Box { direction: dir, sizes: Default::default(), elements: eles }
	}
	/// Create a vertically stacked box
	pub fn new_vert(eles: S) -> Box<S> {
		Box::new(Direction::Vertical, eles)
	}
	/// Create a horizontally stacked box
	pub fn new_horiz(eles: S) -> Box<S> {
		Box::new(Direction::Horizontal, eles)
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

	fn get_rect(&self, ofs: u32, size: u32) -> Rect<::geom::Px> {
		if self.direction.is_vert() {
			Rect::new(0, ofs, !0, size)
		} else {
			Rect::new(ofs, 0, size, !0)
		}
	}
}
impl<S: BoxEleSet> super::Element for Box<S>
{
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool {
		false
	}

	fn element_at_pos(&self, x: u32, y: u32) -> (&Element, (u32,u32))
	{
		let (_cap, exp) = *self.sizes.borrow();

		self.elements.get_ele_at_pos( x, y, exp, self.direction.is_vert() )
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		// 1. Determine sizes
		let (is_dirty, expand_size) = self.update_size(if self.direction.is_vert() { surface.height() } else { surface.width() });

		// 2. Render sub-surfaces
		self.elements.render(surface, force || is_dirty, expand_size, |ofs,size| self.get_rect(ofs, size));
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
	fn get_ele_at_pos(&self, x: u32, y: u32, exp: u32, is_vert: bool) -> (&Element,(u32,u32));
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

			fn get_ele_at_pos(&$s, x: u32, y: u32, exp: u32, is_vert: bool) -> (&Element,(u32,u32)) {
				let pos = if is_vert { y } else { x };
				let mut ofs = 0;
				$({
					let size = if let Some(Size(s)) = $v.size { s } else { exp };
					// If the cursor was before the right/bottom border of this element, it's within
					// - Works because of ordering
					if pos < ofs + size
					{
						return if is_vert {
								$v.ele.element_at_pos(x, y - ofs)
							}
							else {
								$v.ele.element_at_pos(x - ofs, y)
							};
					}
					else {
						ofs += size;
					}
				})*
				let _ = ofs;
				static EMPTY_ELE: () = ();
				(&EMPTY_ELE, (0,0))
			}
		}
		};
}

impl_box_set_tuple!{ self : A = self.0 }
impl_box_set_tuple!{ self : A = self.0, B = self.1 }
impl_box_set_tuple!{ self : A = self.0, B = self.1, C = self.2 }
impl_box_set_tuple!{ self : A = self.0, B = self.1, C = self.2, D = self.3 }
impl_box_set_tuple!{ self : A = self.0, B = self.1, C = self.2, D = self.3, E = self.4 }

