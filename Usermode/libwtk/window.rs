
use geom::Rect;

use syscalls::Object;

pub struct Window<'a>
{
	win: ::syscalls::gui::Window,
	surface: ::surface::Surface,
	root: &'a ::Element,

	focus: Option<&'a ::Element>,
	taborder: Vec<&'a ::Element>,
}

impl<'a> Window<'a>
{
	pub fn new(ele: &::Element) -> Window {
		Window {
			win: match ::syscalls::gui::Window::new("")
				{
				Ok(w) => w,
				Err(e) => panic!("TODO: Window::new e={:?}", e),
				},
			surface: Default::default(),
			root: ele,
			focus: None,
			taborder: Vec::new(),
		}
	}

	pub fn focus(&mut self, ele: &'a ::Element) {
		self.focus.map(|e| e.focus_change(false));
		self.focus = Some(ele);
		ele.focus_change(true);
	}
	pub fn taborder_add(&mut self, ele: &'a ::Element) {
		self.taborder.push( ele );
	}

	pub fn undecorate(&mut self) {
		//panic!("TODO: undecorate");
	}
	pub fn maximise(&mut self) {
		self.win.maximise();
		self.surface.resize( self.win.get_dims() );
	}

	pub fn rerender(&self) {
		self.root.render( self.surface.slice( Rect::new_full() ) );
		self.surface.blit_to_win( &self.win );
	}

	pub fn show(&mut self) {
		self.rerender();
		self.win.show();
	}
}

impl<'a> ::async::WaitController for Window<'a>
{
	fn get_count(&self) -> usize {
		1
	}
	fn populate(&self, cb: &mut FnMut(::syscalls::WaitItem)) {
		cb( self.win.get_wait() )
	}
	fn handle(&mut self, events: &[::syscalls::WaitItem]) {
		let mut redraw = false;
		if self.win.check_wait_input(&events[0])
		{
			while let Some(ev) = self.win.pop_event()
			{
				match ev
				{
				// Capture the Tab key for tabbing between fields
				// TODO: Allow the element to capture instead, maybe by passing self to it?
				::InputEvent::KeyDown(::syscalls::gui::KeyCode::Tab) => {},
				::InputEvent::KeyUp(::syscalls::gui::KeyCode::Tab) => {
					let e = self.taborder[1];	// HACK! Until I cbf tracking the position in the taborder, just hard-code to #2
					self.focus(e);
					redraw = true;
					},
				ev @ _ => 
					if let Some(ele) = self.focus {
						redraw |= ele.handle_event(ev);
					},
				}
			}
		}

		if redraw {
			self.rerender();
			self.win.redraw();
		}
	}
}

