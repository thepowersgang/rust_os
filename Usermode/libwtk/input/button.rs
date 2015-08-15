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
}

impl<'a, T: ::Element> ::Element for Button<'a, T>
{
	fn focus_change(&self, have: bool) {
		self.state.borrow_mut().is_focussed = have;
	}

	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool {
		match ev
		{
		::InputEvent::MouseUp(0) => {
			self.state.borrow_mut().is_held = false;
			true
			},
		::InputEvent::MouseDown(0) => {
			self.state.borrow_mut().is_held = true;
			true
			},
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Return) =>
			if let Some(ref cb) = self.click_cb
			{
				cb(self, win);
				true
			}
			else {
				false
			},
		_ => false,
		}
	}

	fn render(&self, surface: ::surface::SurfaceView) {
		self.inner.render(surface)
	}
}

