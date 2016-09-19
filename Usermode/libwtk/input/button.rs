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

#[cfg(nightly)]
pub type ButtonBcb<'a, T> = Button<T, BoxCb<'a, T>>;

#[cfg(nightly)]
/// Wrapper around a Box<Fn> that allows a `Button` to be stored in a struct
pub struct BoxCb<'a, T: 'a + ::Element>(Box<Fn(&Button<T,BoxCb<'a,T>>, &mut ::window::WindowTrait)+'a>);

impl<'a, 'b1, 'b2, 'b3, T> ::std::ops::Fn<(&'b1 Button<T, BoxCb<'a, T>>, &'b2 mut (::window::WindowTrait<'b3> + 'b2))> for BoxCb<'a, T>
where
	T: 'a + ::Element
{
	extern "rust-call" fn call(&self, args: (&'b1 Button<T, BoxCb<'a, T>>, &'b2 mut (::window::WindowTrait<'b3> + 'b2))) {
		self.0.call(args)
	}
}
#[cfg(nightly)]
impl<'a, 'b1, 'b2, 'b3, T> ::std::ops::FnMut<(&'b1 Button<T, BoxCb<'a, T>>, &'b2 mut (::window::WindowTrait<'b3> + 'b2))> for BoxCb<'a, T>
where
	T: 'a + ::Element
{
	extern "rust-call" fn call_mut(&mut self, args: (&'b1 Button<T, BoxCb<'a, T>>, &'b2 mut (::window::WindowTrait<'b3> + 'b2))) {
		self.call(args)
	}
}
#[cfg(nightly)]
impl<'a, 'b1, 'b2, 'b3, T> ::std::ops::FnOnce<(&'b1 Button<T, BoxCb<'a, T>>, &'b2 mut (::window::WindowTrait<'b3> + 'b2))> for BoxCb<'a, T>
where
	T: 'a + ::Element
{
	type Output = ();
	extern "rust-call" fn call_once(self, args: (&'b1 Button<T, BoxCb<'a, T>>, &'b2 mut (::window::WindowTrait<'b3> + 'b2))) {
		self.call(args)
	}
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
}
impl<'a, T> Button<T, BoxCb<'a, T>>
where
	T: ::Element,
{
	#[cfg(nightly)]
	pub fn new_boxfn<F2>(ele: T, cb: F2) -> Self
	where
		F2: 'a + Fn(&Button<T, BoxCb<'a, T>>, &mut ::window::WindowTrait)
	{
		Button::new(ele, BoxCb(Box::new(cb)))
	}
}

impl<T, F> Button<T, F>
where
	T: ::Element,
	F: Fn(&Button<T, F>, &mut ::window::WindowTrait)
{
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
	fn resize(&self, w: u32, h: u32) {
		self.inner.resize(w, h)
	}
	fn with_element_at_pos(&self, pos: ::geom::PxPos, _dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool {
		f(self, pos)
		//self.inner.with_element_at_pos(pos, f)	// Nah
	}
}

