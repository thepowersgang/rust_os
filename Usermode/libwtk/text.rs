//
//
//
use surface::Colour;
use geom::Rect;

pub struct Label<'a>
{
	colour: Colour,
	value: &'a str,
}

impl<'a> Label<'a>
{
	pub fn new(s: &'a str, colour: Colour) -> Label<'a> {
		Label {
			colour: colour,
			value: s,
		}
	}
}

impl<'a> ::Element for Label<'a>
{
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		if force {
			surface.draw_text( Rect::new(0,0,!0,!0), self.value.chars(), self.colour );
		}
	}
	fn resize(&self, _w: u32, _h: u32) {
	}
	fn with_element_at_pos(&self, pos: ::geom::PxPos, _dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool {
		f(self, pos)
	}
}

