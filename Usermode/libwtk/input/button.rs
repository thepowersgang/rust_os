//
//
//
use std::cell::RefCell;

pub struct Button<T, F>
where
	T: ::Element,
	F: Fn(&Button<T,F>, &mut ::window::WindowTrait)
{
	inner: T,
	click_cb: F,
	state: RefCell<State>,
}

#[derive(Default)]
struct State
{
	is_dirty: bool,

	is_focussed: bool,
	is_held: bool,
}

impl<T, F> Button<T, F>
where
	T: ::Element,
	F: Fn(&Button<T, F>, &mut ::window::WindowTrait)
{
	pub fn new(ele: T, cb: F) -> Button<T, F> {
		Button {
			inner: ele,
			click_cb: cb,
			state: Default::default(),
		}
	}

	pub fn inner(&self) -> &T { &self.inner }
	pub fn inner_mut(&mut self) -> &mut T { &mut self.inner }

	pub fn downstate_change(&self, state: bool) -> bool {
		let mut st = self.state.borrow_mut();
		if st.is_held != state {
			st.is_held = state;
			st.is_dirty = true;
			true
		}
		else {
			false
		}
	}
}

impl<T, F> ::Element for Button<T,F>
where
	T: ::Element,
	F: Fn(&Button<T, F>, &mut ::window::WindowTrait)
{
	fn focus_change(&self, have: bool) {
		let mut st = self.state.borrow_mut();
		st.is_focussed = have;
		st.is_dirty = true;
	}

	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::WindowTrait) -> bool {
		match ev
		{
		::InputEvent::MouseDown(_x,_y,0) => self.downstate_change(true),
		::InputEvent::MouseUp(_x,_y,0) => {
			(self.click_cb)(self, win);
			self.downstate_change(false)
			}
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Return) => {
			(self.click_cb)(self, win);
			false
			},
		_ => false,
		}
	}

	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		//if force || self.state.borrow().is_dirty {
		//	// TODO: Draw a border according using is_focussed and is_held
		//}
		self.inner.render(surface, force)
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::Element, (u32,u32)) {
		(self, (0,0))
		//self.inner.element_at_pos(x, y)
	}
}

