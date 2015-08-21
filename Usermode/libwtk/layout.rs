//
//
//
//! Layout widgets

use surface::Colour;
use super::Element;
use geom::Rect;

#[derive(PartialEq,Debug)]
enum Direction { Vertical, Horizontal }
impl Default for Direction { fn default() -> Direction { Direction::Vertical } }
impl Direction {
	fn is_vert(&self) -> bool { match self { &Direction::Vertical => true, &Direction::Horizontal => false } }
}

pub struct Size(u32);

/// Box containing multiple elements, handles auto-sizing of elements
#[derive(Default)]
pub struct Box<'a>
{
	direction: Direction,
	sizes: ::std::cell::RefCell<(u32,u32)>,
	items: Vec< (Option<&'a Element>, Option<Size>) >,
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
	pub fn add(&mut self, item: &'a Element, size: Option<u32>) {
		self.items.push( (Some(item), size.map(|v| Size(v))) );
	}
	/// Add a spacer to the box, of an optional size
	pub fn add_fill(&mut self, size: Option<u32>) {
		self.items.push( (None, size.map(|v| Size(v))) );
	}

	// returns (has_changed, expand_size)
	fn update_size(&self, cap: u32) -> (bool, u32) {
		let mut sizes = self.sizes.borrow_mut();
		if sizes.0 == cap {
			(false, sizes.1)
		}
		else {
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

impl<'a> super::Element for Box<'a>
{
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool {
		false
	}

	fn element_at_pos(&self, x: u32, y: u32) -> (&Element, (u32,u32))
	{
		let pos = if self.direction.is_vert() { y } else { x };
		let (_cap, exp) = *self.sizes.borrow();

		let mut ofs = 0;
		for &(element, ref size) in self.items.iter()
		{
			let size = if let &Some(ref s) = size { s.0 } else { exp };
			// If the cursor was before the right/bottom border of this element, it's within
			// - Works because of ordering
			if pos < ofs + size
			{
				if let Some(e) = element {
					return if self.direction.is_vert() {
							e.element_at_pos(x, y - ofs)
						}
						else {
							e.element_at_pos(x - ofs, y)
						};
				}
				else {
					break ;
				}
			}
			ofs += size;
		}
		(self,(0,0))
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		// 1. Determine sizes
		let (is_dirty, expand_size)  = self.update_size(if self.direction.is_vert() { surface.height() } else { surface.width() });

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
			Some(ele) => {
				let rect = self.get_rect(ofs, size);
				//kernel_log!("- rect = {:?}", rect);
				ele.render(surface.slice(rect), force || is_dirty);
				},
			None => {},
			}

			ofs += size;
		}
	}
}

enum FrameType { Raise, Bevel }

/// Provides a frame around an element
pub struct Frame<E: ::Element>
{
	item: E,
	frame_type: FrameType,
	//frame_width: u32,
}


impl<E: ::Element> Frame<E>
{
	/// Construct a new framed element
	pub fn new(ele: E) -> Frame<E> {
		Frame {
			frame_type: FrameType::Raise,
			//frame_width: 2, // 2 px of frame
			item: ele,
		}
	}

	pub fn inner(&self) -> &E { &self.item }
	pub fn inner_mut(&mut self) -> &mut E { &mut self.item }
}

impl<E: ::Element> ::Element for Frame<E>
{
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool {
		// TODO: For mouse events, clip to display region
		//
		self.item.handle_event(ev, win)
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool)
	{
		if force
		{
			match self.frame_type
			{
			FrameType::Raise => {
				surface.fill_rect( Rect::new(0,0,!0,1), Colour::theme_border_alt() );
				surface.fill_rect( Rect::new(0,0,1,!0), Colour::theme_border_alt() );
				surface.fill_rect( Rect::new(0,surface.height()-1,!0,1), Colour::theme_border_main() );
				surface.fill_rect( Rect::new(surface.width()-1,0,1,!0), Colour::theme_border_main() );
				},
			FrameType::Bevel => {
				},
			}
		}

		self.item.render(surface.slice( Rect::new(2,2, surface.width()-4, surface.height()-4) ), force);
	}
	fn element_at_pos(&self, x: u32, y: u32) -> (&::Element, (u32,u32))
	{
		if x < 2 || y < 2 {
			(self, (0,0))
		}
		else {
			self.item.element_at_pos(x,y)
		}
	}
}
