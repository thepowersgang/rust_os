
use geom::Rect;

use syscalls::Object;

/// Toolkit window
pub struct Window<'a>
{
	win: ::syscalls::gui::Window,
	surface: ::surface::Surface,
	root: &'a ::Element,

	needs_force_rerender: bool,
	focus: Option<&'a ::Element>,
	taborder: Vec<(usize, &'a ::Element)>,
}

impl<'a> Window<'a>
{
	/// Create a new window containing the provided element
	pub fn new(ele: &::Element) -> Window {
		Window {
			win: match ::syscalls::gui::Window::new("")
				{
				Ok(w) => w,
				Err(e) => panic!("TODO: Window::new e={:?}", e),
				},
			surface: Default::default(),
			root: ele,
			needs_force_rerender: false,
			focus: None,
			taborder: Vec::new(),
		}
	}

	/// Set the currently focussed element to an arbitary element)
	/// 
	/// NOTE: Undefined things happen if this element isn't within this window's 
	///       render tree.
	pub fn focus(&mut self, ele: &'a ::Element) {
		self.focus.map(|e| e.focus_change(false));
		self.focus = Some(ele);
		ele.focus_change(true);
	}
	/// Clear focus
	pub fn clear_focus(&mut self) {
		self.focus.map(|e| e.focus_change(false));
		self.focus = None;
	}

	/// Add the specified element to the tab order. Index uniquely identifies this element in the order
	pub fn taborder_add(&mut self, idx: usize, ele: &'a ::Element) {
		match self.taborder.binary_search_by(|v| ::std::cmp::Ord::cmp(&v.0, &idx))
		{
		Ok(_i) => {
			panic!("TODO: Handle duplicate in tab oredr");
			},
		Err(i) => {
			self.taborder.insert(i, (idx, ele));
			},
		}
	}
	/// Move to the specified location in the tab order (using the index passed to `taborder_add`)
	pub fn tabto(&mut self, idx: usize) {
		if let Ok(i) = self.taborder.binary_search_by(|v| ::std::cmp::Ord::cmp(&v.0, &idx))
		{
			let e = self.taborder[i].1;
			self.focus(e);
		}
	}

	/// Disable window decorations on this window
	pub fn undecorate(&mut self) {
		//panic!("TODO: undecorate");
		self.needs_force_rerender = true;
	}
	/// Maximise the window
	pub fn maximise(&mut self) {
		self.needs_force_rerender = true;
		self.win.maximise();
		self.surface.resize( self.win.get_dims() );
	}

	/// Manually request a redraw of the window
	pub fn rerender(&mut self) {
		self.root.render( self.surface.slice( Rect::new_full() ), self.needs_force_rerender );
		self.surface.blit_to_win( &self.win );
		self.needs_force_rerender = false;
	}

	/// Show the window
	pub fn show(&mut self) {
		self.needs_force_rerender = true;
		self.surface.invalidate_all();
		self.rerender();
		self.win.show();
	}
	pub fn hide(&mut self) {
		self.win.hide();
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
					let e = self.taborder[1].1;	// HACK! Until I cbf tracking the position in the taborder, just hard-code to #2
					self.focus(e);
					redraw = true;
					},
				// Mouse events need to be dispatched correctly
				::InputEvent::MouseMove(x,y,dx,dy) => {
					let (ele, (basex, basey)) = self.root.element_at_pos(x,y /*, self.surface.width(), self.surface.height()*/);
					assert!(x >= basex); assert!(y >= basey);
					// TODO: Also send an event to the source window
					redraw |= ele.handle_event( ::InputEvent::MouseMove(x - basex, y - basey, dx, dy), self );
					},
				::InputEvent::MouseUp(x,y,btn) => {
					let (ele, (basex, basey)) = self.root.element_at_pos(x,y /*, self.surface.width(), self.surface.height()*/);
					assert!(x >= basex); assert!(y >= basey);
					redraw |= ele.handle_event( ::InputEvent::MouseUp(x - basex, y - basey, btn), self );
					},
				::InputEvent::MouseDown(x,y,btn) => {
					let (ele, (basex, basey)) = self.root.element_at_pos(x,y /*, self.surface.width(), self.surface.height()*/);
					assert!(x >= basex); assert!(y >= basey);
					redraw |= ele.handle_event( ::InputEvent::MouseDown(x - basex, y - basey, btn), self );
					},
				ev @ _ => 
					if let Some(ele) = self.focus {
						redraw |= ele.handle_event(ev, self);
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

