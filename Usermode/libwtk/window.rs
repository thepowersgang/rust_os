//
//
//
//
use geom::Rect;
use syscalls::Object;
use syscalls::gui::KeyCode;

// TODO: I would _love_ to make Window generic over the root element, but due to the borrows in `focus`,`taborder`, and `shortcuts_0`, this is nigh-on-impossible
// - Especially if you want &mut to the root

/// Toolkit window
pub struct Window<'a>
{
	win: ::syscalls::gui::Window,
	surface: ::surface::Surface,

	needs_force_rerender: bool,
	focus: Option<&'a ::Element>,
	taborder_pos: usize,
	taborder: Vec<(usize, &'a ::Element)>,

	// Keyboard shortcuts
	//modifier_states: ModiferStates,
	//shortcuts: ::std::collections::HashMap< (KeyCode,Modifiers), Box<FnMut()+'a> >,
	shortcuts_0: Vec<(KeyCode, Box<FnMut()+'a>)>,

	// Rendering information
	background: ::surface::Colour,
	root: &'a ::Element,
}

impl<'a> Window<'a>
{
	/// Create a new window containing the provided element
	pub fn new(debug_name: &str, ele: &'a ::Element, background: ::surface::Colour) -> Window<'a> {
		Window {
			win: match ::syscalls::gui::Window::new(debug_name)
				{
				Ok(w) => w,
				Err(e) => panic!("TODO: Window::new e={:?}", e),
				},
			surface: Default::default(),
			needs_force_rerender: false,
			focus: None,
			taborder_pos: 0,
			taborder: Vec::new(),

			shortcuts_0: Default::default(),

			background: background,
			root: ele,
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
			self.taborder_pos = i;
			let e = self.taborder[i].1;
			self.focus(e);
		}
	}

	/// Add a shortcut key combination
	pub fn add_shortcut<F: 'a + FnMut()>(&mut self, keys: &[KeyCode], fcn: F) /*-> Result<(),()>*/ {
		match keys.len()
		{
		0 => {},
		1 => {
			self.shortcuts_0.push( (keys[0], Box::new(fcn)) );
			},
		_ => {},
		}
	}

	/// Disable window decorations on this window
	pub fn undecorate(&mut self) {
		// TODO: Decide if decoratons should be done client-side, or server-side.
		// - Client is slightly cleaner architectually
		// - Server is more reliable, but has comms costs and server bloat
		self.needs_force_rerender = true;
	}

	pub fn set_pos(&mut self, x: u32, y: u32) {
		self.win.set_pos(x,y)
	}
	pub fn set_dims(&mut self, w: u32, h: u32) {
		self.win.set_dims( ::syscalls::gui::Dims { w: w, h: h } );
		self.update_surface_size();
	}

	/// Maximise the window
	pub fn maximise(&mut self) {
		self.win.maximise();
		self.update_surface_size();
	}
	fn update_surface_size(&mut self) {
		self.needs_force_rerender = true;
		self.surface.resize( self.win.get_dims(), self.background );
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

	fn handle_event(&mut self, ev: ::InputEvent) -> bool {
		kernel_log!("Window::handle_event(ev={:?})", ev);
		match ev
		{
		// Capture the Tab key for tabbing between fields
		// TODO: Allow the element to capture instead, maybe by passing self to it?
		::InputEvent::KeyDown(::syscalls::gui::KeyCode::Tab) => false,
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Tab) => {
			if self.taborder.len() > 0 {
				self.taborder_pos = (self.taborder_pos + 1) % self.taborder.len();
				let e = self.taborder[self.taborder_pos].1;
				self.focus(e);
				true
			}
			else {
				false
			}
			},
		// Mouse events need to be dispatched correctly
		::InputEvent::MouseMove(x,y,dx,dy) => {
			let (ele, (basex, basey)) = self.root.element_at_pos(x,y /*, self.surface.width(), self.surface.height()*/);
			assert!(x >= basex); assert!(y >= basey);
			// TODO: Also send an event to the source element
			ele.handle_event( ::InputEvent::MouseMove(x - basex, y - basey, dx, dy), self )
			},
		::InputEvent::MouseUp(x,y,btn) => {
			let (ele, (basex, basey)) = self.root.element_at_pos(x,y /*, self.surface.width(), self.surface.height()*/);
			assert!(x >= basex); assert!(y >= basey);
			// TODO: Also send MouseUp to the element that received the MouseDown
			ele.handle_event( ::InputEvent::MouseUp(x - basex, y - basey, btn), self )
			},
		::InputEvent::MouseDown(x,y,btn) => {
			let (ele, (basex, basey)) = self.root.element_at_pos(x,y /*, self.surface.width(), self.surface.height()*/);
			assert!(x >= basex); assert!(y >= basey);
			ele.handle_event( ::InputEvent::MouseDown(x - basex, y - basey, btn), self )
			},
		ev @ _ => {
			match ev {
			::InputEvent::KeyUp(key) => {
				for &mut (s_key, ref mut fcn) in self.shortcuts_0.iter_mut() {
					if key == s_key {
						fcn();
						return false;
					}
				}
				},
			_ => {},
			}
			
			if let Some(ele) = self.focus {
				ele.handle_event(ev, self)
			}
			else {
				false
			}
			},
		}
	}
}

impl<'a> ::async::WaitController for Window<'a>
{
	fn get_count(&self) -> usize {
		1
	}
	fn populate(&self, cb: &mut FnMut(::syscalls::WaitItem)) {
		cb( self.win.get_wait( ::syscalls::gui::WindowWaits::new().input() ) )
	}
	fn handle(&mut self, events: &[::syscalls::WaitItem]) {
		let mut redraw = false;
		if self.win.check_wait(&events[0]).has_input()
		{
			while let Some(ev) = self.win.pop_event()
			{
				redraw |= self.handle_event(ev);
			}
		}

		if redraw {
			self.rerender();
			self.win.redraw();
		}
	}
}

