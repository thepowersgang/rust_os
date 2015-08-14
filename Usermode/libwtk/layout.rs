//
//
//
//! Layout widgets

use surface::Colour;
use super::Element;
use geom::Rect;

#[derive(PartialEq,Debug)]
enum Direction { Vertical, Horizontal }
impl Direction {
	fn is_vert(&self) -> bool { match self { &Direction::Vertical => true, &Direction::Horizontal => false } }
}

pub struct Size(u32);

/// Box containing multiple elements, handles auto-sizing of elements
pub struct Box<'a>
{
	direction: Direction,
	items: Vec< (Option<&'a Element>, Option<Size>) >,
}

impl<'a> Box<'a>
{
	/// Create a vertically stacked box
	pub fn new_vert() -> Box<'a> {
		Box { direction: Direction::Vertical, items: Vec::new() }
	}
	/// Create a horizontally stacked box
	pub fn new_horiz() -> Box<'a> {
		Box { direction: Direction::Horizontal, items: Vec::new() }
	}

	/// Add an item to the box, optionally a fixed size
	pub fn add(&mut self, item: &'a Element, size: Option<u32>) {
		self.items.push( (Some(item), size.map(|v| Size(v))) );
	}
	/// Add a spacer to the box, of an optional size
	pub fn add_fill(&mut self, size: Option<u32>) {
		self.items.push( (None, size.map(|v| Size(v))) );
	}
}

impl<'a> super::Element for Box<'a>
{
	fn render(&self, surface: ::surface::SurfaceView) {
		// 1. Determine sizes
		let (fixed_total, num_expand) = self.items.iter().fold( (0,0), |(total,exp), i| if let Some(Size(v)) = i.1 { (total+v, exp) } else { (total, exp+1) } );
		if fixed_total > surface.width() {
			return ;
		}
		let expand_size = if num_expand > 0 {
				( if self.direction.is_vert() { surface.height() } else { surface.width() } - fixed_total) / num_expand
			}
			else {
				0
			};
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
				let rect = if self.direction.is_vert() {
						Rect::new(0, ofs, !0, size)
					} else {
						Rect::new(ofs, 0, size, !0)
					};
				//kernel_log!("- rect = {:?}", rect);
				ele.render(surface.slice(rect));
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
	fn render(&self, surface: ::surface::SurfaceView) {
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

		self.item.render(surface.slice( Rect::new(2,2, surface.width()-4, surface.height()-4) ));
	}
}
