//
//
//
use std::cell::RefCell;

pub struct Button<'a, T: ::Element>
{
	inner: T,
	click_cb: Option< Box<Fn(&Button<'a, T>, &mut ::window::Window)+'a> >,
	state: RefCell<State>,
}

#[derive(Default)]
struct State
{
	is_dirty: bool,

	is_focussed: bool,
	is_held: bool,
}

impl<'a, T: ::Element> Button<'a, T>
{
	pub fn new(ele: T) -> Button<'a, T> {
		Button {
			inner: ele,
			click_cb: None,
			state: Default::default(),
		}
	}

	pub fn inner(&self) -> &T { &self.inner }
	pub fn inner_mut(&mut self) -> &mut T { &mut self.inner }

	pub fn bind_click<F: Fn(&Self, &mut ::window::Window)+'a>(&mut self, cb: F) {
		self.click_cb = Some( Box::new(cb) );
	}

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

impl<'a, T: ::Element> ::Element for Button<'a, T>
{
	fn focus_change(&self, have: bool) {
		let mut st = self.state.borrow_mut();
		st.is_focussed = have;
		st.is_dirty = true;
	}

	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool {
		match ev
		{
		::InputEvent::MouseDown(_x,_y,0) => self.downstate_change(true),
		::InputEvent::MouseUp(_x,_y,0) => self.downstate_change(false),
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Return) => {
			self.click_cb.as_ref().map(|cb| cb(self, win));
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
}

