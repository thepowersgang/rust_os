//
//
//
//! Layout widgets

use surface::Colour;
use super::Element;
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
pub struct Size(u32);

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

impl<'a> super::Element for Box<'a>
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

enum FrameType {
	Raise,
	Ring,
}

/// Provides a frame around an element
pub struct Frame<E: ::Element>
{
	item: E,
	frame_type: FrameType,
	frame_width: u32,
	colour_major: Colour,
	colour_minor: Colour,
}


impl<E: ::Element> Frame<E>
{
	/// Construct a new framed element
	pub fn new_thin(ele: E) -> Frame<E> {
		Frame {
			item: ele,
			frame_type: FrameType::Raise,
			frame_width: 1,
			colour_major: Colour::theme_border_main(),
			colour_minor: Colour::theme_border_alt(),
		}
	}
	pub fn new_fat(ele: E) -> Frame<E> {
		Frame {
			item: ele,
			frame_type: FrameType::Ring,
			frame_width: 3,
			colour_major: Colour::theme_border_main(),
			colour_minor: Colour::theme_border_alt(),
		}
	}

	pub fn inner(&self) -> &E { &self.item }
	pub fn inner_mut(&mut self) -> &mut E { &mut self.item }
}

impl<E: ::Element> ::Element for Frame<E>
{
	fn handle_event(&self, ev: ::InputEvent, win: &mut dyn crate::window::WindowTrait) -> bool {
		// TODO: For mouse events, clip to display region
		//
		self.item.handle_event(ev, win)
	}
	fn resize(&self, w: u32, h: u32) {
		self.item.resize(w - self.frame_width * 2, h - self.frame_width * 2)
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool)
	{
		if force
		{
			match self.frame_type
			{
			FrameType::Raise => {
				let lw = self.frame_width;
				surface.fill_rect( Rect::new(0,0,!0,lw), self.colour_minor );
				surface.fill_rect( Rect::new(0,0,lw,!0), self.colour_minor );
				surface.fill_rect( Rect::new(0,surface.height()-lw,!0,lw), self.colour_major );
				surface.fill_rect( Rect::new(surface.width()-lw,0,lw,!0), self.colour_major );
				},
			FrameType::Ring => {
				let outer_w  = ::geom::Px((self.frame_width + 2) / 3);
				let middle_w = ::geom::Px((self.frame_width + 1) / 3);
				let inner_w  = ::geom::Px((self.frame_width + 0) / 3);

				// Outer
				let mut rect = surface.rect();
				surface.draw_rect( rect, outer_w, self.colour_major );
				rect.x = rect.x + outer_w; rect.w = rect.w - outer_w*2;
				rect.y = rect.y + outer_w; rect.h = rect.h - outer_w*2;
				// Inner
				surface.draw_rect( rect, middle_w, self.colour_minor );
				rect.x = rect.x + middle_w; rect.w = rect.w - middle_w*2;
				rect.y = rect.y + middle_w; rect.h = rect.h - middle_w*2;
				// Middle
				surface.draw_rect( rect, inner_w, self.colour_major );
				},
			}
		}

		let lw = self.frame_width;
		self.item.render(surface.slice( Rect::new(lw+1,lw+1, surface.width()-lw*2-2, surface.height()-lw*2-2) ), force);
	}
	fn with_element_at_pos(&self, pos: ::geom::PxPos, dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool
	{
		if pos.x.0 < 2 || pos.y.0 < 2 {
			f(self, pos)
		}
		else {
			self.item.with_element_at_pos(pos - ::geom::PxPos::new(2,2), dims - ::geom::PxDims::new(2*2,2*2), f)
		}
	}
}
