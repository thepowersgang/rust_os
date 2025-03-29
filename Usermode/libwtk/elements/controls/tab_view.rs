// TODO: Come up with a common way of handling static and dynamic dispatch (and element lists)
// - Option 1: Fixed-size heterogenous element list (tuple)
// - Option 2: Fixed-size homogenous element list (array)
// - Option 3: Dynamic-size homogenous element list (Vec)
// Challenge: The tab bar also wants to know the element count/values

use super::tab_bar::{Position,TabBar};

/// A switchable view on elements, using a tab bar
pub struct TabView
{
	bar: TabBar,
	views: Vec<Box<dyn crate::Element>>,
}
impl TabView
{
	pub fn new(position: Position, bar_size: u32) -> Self {
		TabView {
			bar: TabBar::new(position, bar_size),
			views: Vec::new()
		}
	}
	pub fn new_below() -> Self {
		Self::new(Position::Below, 20)	// HACK: Assume 20px fits standard text
	}
	pub fn new_above() -> Self {
		Self::new(Position::Above, 20)	// HACK: Assume 20px fits standard text
	}

	pub fn selected_idx(&self) -> usize {
		self.bar.selected_idx()
	}

	/// Add a new tab to the end of the list, and return `self`
	pub fn with_tab(mut self, label: impl Into<super::tab_bar::Str>, element: impl crate::Element + 'static) -> Self {
		self.add_tab(label, element);
		self
	}
	/// Add a new tab to the end of the list
	pub fn add_tab(&mut self, label: impl Into<super::tab_bar::Str>, element: impl crate::Element + 'static) {
		self.bar.add_tab(label.into());
		self.views.push(Box::new(element));
	}
}
impl TabView {
	/// Helper: Get the rectangles for the bar and the display (in that order)
	fn get_rects(&self, dims: crate::geom::PxDims) -> [crate::geom::Rect<crate::geom::Px>; 2] {
		let bs = crate::geom::Px(self.bar.size);
		let zero = crate::geom::Px(0);
		match self.bar.position {
		Position::Below => {
			let t = crate::geom::Rect { x: zero, w: dims.w,  y: zero, h: dims.h - bs };
			let b = crate::geom::Rect { x: zero, w: dims.w,  y: dims.h - bs, h: bs };
			[b, t]
		},
		Position::Left => {
			let l = crate::geom::Rect { y: zero, h: dims.h, x: zero , w: bs };
			let r = crate::geom::Rect { y: zero, h: dims.h, x: bs, w: dims.w - bs };
			[l,r]
		},
		Position::Right => {
			let l = crate::geom::Rect { y: zero, h: dims.h, x: zero, w: dims.w - bs };
			let r = crate::geom::Rect { y: zero, h: dims.h, x: dims.w - bs, w: bs };
			[r,l]
		},
		Position::Above => {
			let t = crate::geom::Rect { x: zero, w: dims.w, y: zero, h: bs };
			let b = crate::geom::Rect { x: zero, w: dims.w, y: bs, h: dims.h - bs };
			[t,b]
		},
		}
	}
}

impl crate::Element for TabView {
	fn render(&self, surface: crate::surface::SurfaceView, force: bool) {
		let [r_bar, r_disp] = self.get_rects(crate::geom::PxDims::new(surface.width(), surface.height()));
		self.bar.render(surface.slice(r_bar), force);
		self.views[ self.bar.selected_idx() ].render(surface.slice(r_disp), force);
	}

	fn resize(&self, w: u32, h: u32) {
		let (w,h) = match self.bar.position.is_horiz() {
			true  => { self.bar.resize(w, self.bar.size); (w, h - self.bar.size,) },
			false => { self.bar.resize(self.bar.size, h); (w - self.bar.size, h,) },
			};
		for v in &self.views {
			v.resize(w, h);
		}
	}

	fn with_element_at_pos(&self, pos: crate::geom::PxPos, dims: crate::geom::PxDims, f: crate::WithEleAtPosCb) -> bool {
		let [r_bar, r_disp] = self.get_rects(dims);
		if r_bar.contains(pos) {
			self.bar.with_element_at_pos(pos - r_bar.top_left(), r_bar.dims(), f)
		}
		else {
			self.views[self.bar.selected_idx()].with_element_at_pos(pos - r_disp.top_left(), r_disp.dims(), f)
		}
	}
}
