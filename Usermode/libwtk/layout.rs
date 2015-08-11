
use surface::Colour;
use super::Element;
use geom::Rect;

#[derive(PartialEq,Debug)]
enum Direction { Vertical, Horizontal }
impl Direction {
	fn is_vert(&self) -> bool { match self { &Direction::Vertical => true, &Direction::Horizontal => false } }
}

pub struct Size(u32);

pub struct Box<'a>
{
	direction: Direction,
	items: Vec< (Option<&'a Element>, Option<Size>) >,
}

impl<'a> Box<'a>
{
	pub fn new_vert() -> Box<'a> {
		Box { direction: Direction::Vertical, items: Vec::new() }
	}
	pub fn new_horiz() -> Box<'a> {
		Box { direction: Direction::Horizontal, items: Vec::new() }
	}

	pub fn add(&mut self, item: &'a Element, size: Option<u32>) {
		self.items.push( (Some(item), size.map(|v| Size(v))) );
	}
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

pub struct Frame<'a>
{
	frame_type: FrameType,
	frame_width: u32,
	item: Option<&'a Element>,
}


impl<'a> Frame<'a>
{
	pub fn new() -> Frame<'a> {
		Frame {
			frame_type: FrameType::Raise,
			frame_width: 2, // 2 px of frame
			item: None,
		}
	}

	pub fn add(&mut self, item: &'a Element) {
		assert!(self.item.is_none());
		self.item = Some(item);
	}
}

impl<'a> ::Element for Frame<'a>
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
		match self.item
		{
		Some(i) => i.render(surface.slice( Rect::new(2,2, surface.width()-4, surface.height()-4) )),
		None => {},
		}
	}
}
