//
//
//
//! Layout widgets

use crate::Element;
use geom::Rect;

#[derive(PartialEq,Debug,Copy,Clone)]
enum Direction { Vertical, Horizontal }
impl Default for Direction { fn default() -> Direction { Direction::Vertical } }
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
struct Size(u32);

/// Box containing multiple elements, handles auto-sizing of elements
#[derive(Default)]
pub struct Box<'a>
{
	direction: Direction,
	items: Vec< (Option<&'a mut dyn Element>, Option<Size>) >,
	sizes: ::std::cell::RefCell<SizeState>,
	size_changed: ::std::cell::Cell<bool>,
}

#[derive(Default)]
struct SizeState
{
	last_cap: u32,
	expand_size: u32,
}

impl<'a> Box<'a>
{
	/// Create a vertically stacked box
	pub fn new_vert() -> Box<'a> {
		Box { direction: Direction::Vertical, ..Default::default() }
	}
	/// Create a horizontally stacked box
	pub fn new_horiz() -> Box<'a> {
		Box { direction: Direction::Horizontal, ..Default::default() }
	}

	/// Add an item to the box, optionally a fixed size
	pub fn add(&mut self, item: &'a mut dyn Element, size: Option<u32>) {
		self.items.push( (Some(item), size.map(|v| Size(v))) );
	}
	/// Add a spacer to the box, of an optional size
	pub fn add_fill(&mut self, size: Option<u32>) {
		self.items.push( (None, size.map(|v| Size(v))) );
	}

	// returns (has_changed, expand_size)
	fn update_size(&self, cap: u32) -> (bool, u32) {
		let mut sizes = self.sizes.borrow_mut();
		if sizes.last_cap == cap {
			(false, sizes.expand_size)
		}
		else {
			self.size_changed.set(true);
			let expand = {
				let (fixed_total, num_expand) = self.items.iter().fold( (0,0), |(total,exp), i| if let Some(Size(v)) = i.1 { (total+v, exp) } else { (total, exp+1) } );
				if fixed_total > cap {
					0
				}
				else if num_expand > 0 {
					(cap - fixed_total) / num_expand
				}
				else {
					0
				}
				};
			sizes.last_cap = cap;
			sizes.expand_size = expand;
			(true, expand)
		}
	}
}

impl<'a> Element for Box<'a>
{
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut dyn crate::window::WindowTrait) -> bool {
		false
	}

	fn with_element_at_pos(&self, p: ::geom::PxPos, dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool
	{
		let pos = if self.direction.is_vert() { p.y.0 } else { p.x.0 };
		let SizeState { expand_size, .. } = *self.sizes.borrow();

		let mut ofs = 0;
		for &(ref element, ref size) in self.items.iter()
		{
			let size = if let &Some(ref s) = size { s.0 } else { expand_size };
			// If the cursor was before the right/bottom border of this element, it's within
			// - Works because of ordering
			if pos < ofs + size
			{
				if let &Some(ref e) = element {
					return if self.direction.is_vert() {
							e.with_element_at_pos(p - ::geom::PxPos::new(0,ofs), ::geom::PxDims::new(dims.w.0, size), f)
						}
						else {
							e.with_element_at_pos(p - ::geom::PxPos::new(ofs,0), ::geom::PxDims::new(size, dims.h.0), f)
						};
				}
				else {
					break ;
				}
			}
			ofs += size;
		}
		
		unreachable!()
	}
	fn resize(&self, w: u32, h: u32) {
		let (changed, expand_size) = self.update_size(if self.direction.is_vert() { h } else { w });
		if changed
		{
			for item in self.items.iter()
			{
				let size = item.1.map(|Size(s)| s).unwrap_or(expand_size);

				match item.0
				{
				Some(ref ele) => {
					let rect = self.direction.get_rect(0, size);
					ele.resize(rect.width().0, rect.height().0);
					},
				None => {},
				}
			}
		}
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		// 1. Determine sizes
		let is_dirty = self.size_changed.get(); self.size_changed.set(false);
		let expand_size  = self.sizes.borrow().expand_size;

		// 2. Render sub-surfaces
		let mut ofs = 0;
		for item in self.items.iter()
		{
			let size = match item.1
				{
				Some(Size(size)) => size,
				None => expand_size,
				};
			//kernel_log!("Box::render {:?} - ofs={},size={}", self.direction, ofs, size);

			match item.0
			{
			Some(ref ele) => {
				let rect = self.direction.get_rect(ofs, size);
				//kernel_log!("- rect = {:?}", rect);
				ele.render(surface.slice(rect), force || is_dirty);
				},
			None => {},
			}

			ofs += size;
		}
	}
}
