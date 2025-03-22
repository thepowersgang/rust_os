
/// Common trait for window elements
pub trait Element
{
	/// Called when focus changes to/from this element
	fn focus_change(&self, _have: bool) {
	}
	/// Called when an event fires. Keyboard events are controlled by focus, mouse via the render tree
	fn handle_event(&self, _ev: crate::InputEvent, _win: &mut dyn crate::window::WindowTrait) -> bool {
		false
	}

	/// Redraw this element into the provided surface view
	// MEMO: Cannot take &mut, because that requires `root: &mut` in Window, which precludes passing &mut Window to Element::handle_event
	fn render(&self, surface: crate::surface::SurfaceView, force: bool);

	/// Update size-based information (should be called before a render with a new size, and may be expensive)
	fn resize(&self, _w: u32, _h: u32);

	/// Fetch child element at the given position.
	/// Returns the child element and the offset of the child.
	fn with_element_at_pos(&self, pos: crate::geom::PxPos, dims: crate::geom::PxDims, f: crate::WithEleAtPosCb) -> bool;// { f(self, pos) }
}
macro_rules! dispatch_ele {
	($self_:ident, $inner:expr) => {
		fn focus_change(&$self_, have: bool) { $inner.focus_change(have) }
		fn handle_event(&$self_, ev: crate::InputEvent, win: &mut dyn crate::window::WindowTrait) -> bool { $inner.handle_event(ev, win) }

		fn render(&$self_, surface: crate::surface::SurfaceView, force: bool) { $inner.render(surface, force) }
		fn resize(&$self_, w: u32, h: u32) { $inner.resize(w, h) }
		fn with_element_at_pos(&$self_, pos: crate::geom::PxPos, dims: crate::geom::PxDims, f: crate::WithEleAtPosCb) -> bool { $inner.with_element_at_pos(pos,dims,f) }
		};
}
/// Object safe
impl<'a, T: 'a + Element> Element for &'a T
{
	dispatch_ele!{self, *self}
}
/// RefCell - Interior mutability
impl<T: Element> Element for crate::std::cell::RefCell<T>
{
	dispatch_ele!{self, self.borrow()}
}
/// Unit type is a valid element. Just does nothing.
impl Element for ()
{
	fn focus_change(&self, _have: bool) { }
	fn handle_event(&self, _ev: crate::InputEvent, _win: &mut dyn crate::window::WindowTrait) -> bool { false }
	fn render(&self, _surface: crate::surface::SurfaceView, _force: bool) { }
	fn resize(&self, _w: u32, _h: u32) { }
	fn with_element_at_pos(&self, pos: crate::geom::PxPos, _dims: crate::geom::PxDims, f: crate::WithEleAtPosCb) -> bool { f(self, pos) }
}