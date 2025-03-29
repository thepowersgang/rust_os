
pub type Str = ::std::borrow::Cow<'static, str>;

/// Position/orientation of the view (i.e. to which side of the content)
pub enum Position {
	Below,
	Left,
	Right,
	Above,
}
impl Position {
	pub fn is_horiz(&self) -> bool {
		match self {
		Position::Below|Position::Above => true,
		Position::Left |Position::Right => false,
		}
	}
}

/// Just the bar part of a tab view
pub struct TabBar
{
	/// Height or width, depending on the position
	pub size: u32,
	/// Position of the bar relative to the content it controls
	pub position: Position,

	/// Tab list (labels)
	labels: Vec<Str>,

	state: TabBarState,
}
#[derive(Default)]
struct TabBarState {
	selected: ::std::cell::Cell<usize>,
	last_selected: ::std::cell::Cell<usize>,
}
impl TabBar
{
	pub fn new(position: Position, size: u32) -> Self {
		TabBar {
			size,
			position,
			labels: Vec::new(),

			state: Default::default(),
		}
	}
	/// Add a new tab to the end of the list
	pub fn add_tab(&mut self, label: super::tab_bar::Str) {
		self.labels.push(label);
	}
	pub fn selected_idx(&self) -> usize {
		self.state.selected.get()
	}
}
impl crate::Element for TabBar {
	fn render(&self, surface: crate::surface::SurfaceView, mut force: bool) {
		use crate::geom::{Rect,PxDims, Px};
		use crate::Colour;

		if self.state.selected != self.state.last_selected {
			self.state.last_selected.set(self.state.selected.get());
			force = true;
		}
		if force {
			surface.fill_rect(Rect::new(0,0,!0,!0), Colour::theme_body_bg());
		}
		
		let mut rect = Rect::new(0, 0, !0, !0);
		for (i,label) in self.labels.iter().enumerate() {
			let (w,h) = crate::surface::SurfaceView::size_text(label.chars());
			// Align the text with middle-left of the tab?
			surface.draw_text(rect, label.chars(), Colour::theme_text());

			let (l_t,l_r,l_b,l_l) = if i == self.selected_idx() {
					// If this is the selected element, omit an edge
					match self.position {
					Position::Below => (false, true, true, true),
					Position::Left => (true, false, true, true),
					Position::Right => (true, true, true, false),
					Position::Above => (true, true, false, true),
					}
				}
				else {
					(true,true,true,true)
				};

			let r= if self.position.is_horiz() {
				let r = rect.with_dims(PxDims::new(w, self.size));
				rect.x.0 += w;
				r
			}
			else {
				let r = rect.with_dims(PxDims::new(self.size, h));
				rect.y.0 += h;
				r
			};
			if l_l {
				surface.fill_rect(Rect { w: Px(1), ..r }, Colour::theme_border_main());
			}
			if l_t {
				surface.fill_rect(Rect { h: Px(1), ..r }, Colour::theme_border_main());
			}
			if l_r {
				surface.fill_rect(Rect { w: Px(1), x: r.x + r.w - Px(1), ..r }, Colour::theme_border_main());
			}
			if l_b {
				surface.fill_rect(Rect { h: Px(1), y: r.y + r.h - Px(1), ..r }, Colour::theme_border_main());
			}
		}
	}

	fn resize(&self, _w: u32, _h: u32) {
		// Nothing much to do on resize
	}

	fn with_element_at_pos(&self, pos: crate::geom::PxPos, _dims: crate::geom::PxDims, f: crate::WithEleAtPosCb) -> bool {
		// TODO: If elements are used for labels instead, then this needs to account for that AND also pass to itself
		f(self, pos)
	}

	fn focus_change(&self, _have: bool) {
		// TODO: Focus selection (arrow keys)
	}
	fn handle_event(&self, ev: crate::InputEvent, _win: &mut dyn crate::window::WindowTrait) -> bool {
		use crate::InputEvent as Ev;
		match ev {
		Ev::MouseClick(mut x, mut y, 0) => {
			for (i,label) in self.labels.iter().enumerate() {
				let (w,h) = crate::surface::SurfaceView::size_text(label.chars());
				if self.position.is_horiz() {
					if x < w {
						self.state.selected.set(i);
						break;
					}
					x -= w;
				}
				else {
					if y < h {
						self.state.selected.set(i);
						break;
					}
					y -= h;
				}
			}
			true
		},
		_ => false,
		}
	}
}