use crate::Element;
use crate::surface::Colour;
use crate::geom::Rect;

enum FrameType {
	Raise,
	Ring,
}

/// Provides a frame around an element
pub struct Frame<E: Element>
{
	item: E,
	frame_type: FrameType,
	frame_width: u32,
	colour_major: Colour,
	colour_minor: Colour,
}


impl<E: Element> Frame<E>
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

impl<E: Element> Element for Frame<E>
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
