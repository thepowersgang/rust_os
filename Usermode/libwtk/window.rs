//
//
//
//! WTK Window
use geom::{Rect,Pos,Dims};
use syscalls::Object;
use syscalls::gui::KeyCode;
use decorator::Decorator;

#[derive(Debug)]
pub struct Error;

// TODO: I would _love_ to make Window generic over the root element, but due to the borrows in `focus`,`taborder`, and `shortcuts_0`, this is nigh-on-impossible
// - Especially if you want &mut to the root

pub trait WindowTrait<'a>
{
	fn set_title(&mut self, title: String);
	/// Set window position
	fn set_pos(&mut self, x: u32, y: u32);
	/// Set window dimensions (excludes decorator area), may be restricted by server
	fn set_dims(&mut self, w: u32, h: u32);
	fn get_dims(&self) -> (u32, u32);
	fn get_full_dims(&self) -> (u32, u32);
	/// Maximise the window
	fn maximise(&mut self);
	/// Show the window
	fn show(&mut self);
	/// Hide the window
	fn hide(&mut self);
	/// Set the currently focussed element to an arbitary element)
	/// 
	/// NOTE: Undefined things happen if this element isn't within this window's 
	///       render tree.
	fn focus(&mut self, ele: &'a dyn crate::Element);
	/// Clear focus
	fn clear_focus(&mut self);
	/// Move to the specified location in the tab order (using the index passed to `taborder_add`)
	fn tabto(&mut self, idx: usize);
	/// Manually request a redraw of the window
	fn rerender(&mut self);

	/// Obtain the states of all "modifier" keys
	fn get_modifiers(&self) -> &ModifierStates;
}

/// Toolkit window
pub struct Window<'a, D: 'a/* = ::decorator::Standard*/>
{
	win: ::syscalls::gui::Window,
	surface: ::surface::Surface,

	needs_force_rerender: bool,
	focus: Option<&'a dyn crate::Element>,
	taborder_pos: usize,
	taborder: Vec<(usize, &'a dyn crate::Element)>,

	// Keyboard shortcuts
	modifier_states: ModifierStates,
	//shortcuts: ::std::collections::HashMap< (KeyCode,Modifiers), Box<FnMut()+'a> >,
	shortcuts: Vec< ( (KeyCode,Modifiers), Box<dyn FnMut()+'a> ) >,
	shortcuts_0: Vec<(KeyCode, Box<dyn FnMut()+'a>)>,

	// Rendering information
	background: ::surface::Colour,
	root: &'a dyn crate::Element,
	pub decorator: D,
}

impl<'a> Window<'a, ::decorator::Standard>
{
	pub fn new_def(debug_name: &str, ele: &'a dyn crate::Element) -> Result<Self, Error> {
		Window::new(debug_name, ele, ::surface::Colour::from_argb32(0), ::decorator::Standard::default())
	}
}
impl<'a, D: 'a + Decorator> Window<'a, D>
{
	/// Create a new window containing the provided element
	pub fn new(debug_name: &str, ele: &'a dyn crate::Element, background: ::surface::Colour, decorator: D) -> Result<Window<'a, D>, Error> {
		let w = match ::syscalls::gui::Window::new(debug_name)
			{
			Ok(w) => w,
			Err(e) => panic!("TODO: Window::new e={:?}", e),
			};
		Ok(Window {
			win: w,
			surface: Default::default(),
			needs_force_rerender: false,
			focus: None,
			taborder_pos: 0,
			taborder: Vec::new(),

			modifier_states: Default::default(),
			shortcuts: Default::default(),
			shortcuts_0: Default::default(),

			background: background,
			root: ele,
			decorator: decorator,
		})
	}

	/// Add the specified element to the tab order. Index uniquely identifies this element in the order
	pub fn taborder_add(&mut self, idx: usize, ele: &'a dyn crate::Element) {
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

	/// Add a shortcut key combination
	pub fn add_shortcut_1<F: 'a + FnMut()>(&mut self, key: KeyCode, fcn: F) /*-> Result<(),()>*/ {
		self.shortcuts_0.push( (key, Box::new(fcn)) );
	}
	pub fn add_shortcut_2<F: 'a + FnMut()>(&mut self, m: Modifier, key: KeyCode, fcn: F) /*-> Result<(),()>*/ {
		self.shortcuts.push( ( (key, Modifiers::new(&[m])), Box::new(fcn)) );
	}
	pub fn rm_shortcut_1(&mut self, key: KeyCode) {
		self.shortcuts_0.retain(|s| s.0 != key);
	}
	pub fn rm_shortcut_2(&mut self, m: Modifier, key: KeyCode) {
		self.shortcuts.retain(|s| s.0 != (key, Modifiers::new(&[m])));
	}

	pub fn idle_loop(&mut self) {
		::r#async::idle_loop(&mut [ self ]);
	}
}

impl<'a, D: 'a + Decorator> Window<'a, D>
{
	pub fn set_title<T: Into<String>>(&mut self, title: T) {
		WindowTrait::set_title(self, title.into())
	}
	/// Set window position
	pub fn set_pos(&mut self, x: u32, y: u32) {
		WindowTrait::set_pos(self, x, y)
	}
	/// Set window dimensions (excludes decorator area), may be restricted by server
	pub fn set_dims(&mut self, w: u32, h: u32) {
		WindowTrait::set_dims(self, w, h)
	}
	/// Maximise the window
	pub fn maximise(&mut self) {
		WindowTrait::maximise(self)
	}
	/// Show the window
	pub fn show(&mut self) {
		WindowTrait::show(self)
	}
	/// Hide the window
	pub fn hide(&mut self) {
		WindowTrait::hide(self)
	}
	/// Set the currently focussed element to an arbitary element)
	pub fn focus(&mut self, ele: &'a dyn crate::Element) {
		WindowTrait::focus(self, ele)
	}
	/// Clear focus
	pub fn clear_focus(&mut self) {
		WindowTrait::clear_focus(self)
	}
	/// Move to the specified location in the tab order (using the index passed to `taborder_add`)
	pub fn tabto(&mut self, idx: usize) {
		WindowTrait::tabto(self, idx)
	}
	/// Manually request a redraw of the window
	pub fn rerender(&mut self)  {
		WindowTrait::rerender(self)
	}

	/// Obtain the states of all "modifier" keys
	pub fn get_modifiers(&self) -> &ModifierStates {
		WindowTrait::get_modifiers(self)
	}
}

impl<'a, D: 'a + Decorator> WindowTrait<'a> for Window<'a, D>
{
	fn set_title(&mut self, title: String) {
		self.decorator.set_title(title);
	}

	/// Set window position
	fn set_pos(&mut self, x: u32, y: u32) {
		self.win.set_pos(x,y)
	}
	/// Set window dimensions (excludes decorator area), may be restricted by server
	fn set_dims(&mut self, w: u32, h: u32) {
		let (decor_tl, decor_br) = self.decorator.client_rect();
		let w = w + decor_tl.w.0 + decor_br.w.0;
		let h = h + decor_tl.h.0 + decor_br.h.0;
		self.win.set_dims( ::syscalls::gui::Dims { w: w, h: h } );
		self.update_surface_size();
	}
	fn get_dims(&self) -> (u32, u32) {
		let (decor_tl, decor_br) = self.decorator.client_rect();
		let Dims { w, h } = self.surface.rect().dims();
		(
			w.0 - decor_tl.w.0 + decor_br.w.0,
			h.0 - decor_tl.h.0 + decor_br.h.0,
			)
	}
	fn get_full_dims(&self) -> (u32, u32) {
		let Dims { w, h } = self.surface.rect().dims();
		(w.0, h.0)
	}

	/// Maximise the window
	fn maximise(&mut self) {
		self.win.maximise();
		self.update_surface_size();
	}

	/// Show the window
	fn show(&mut self) {
		self.needs_force_rerender = true;
		self.surface.invalidate_all();
		self.rerender();
		self.win.show();
	}

	/// Hide the window
	fn hide(&mut self) {
		self.win.hide();
	}

	/// Set the currently focussed element to an arbitary element)
	/// 
	/// NOTE: Undefined things happen if this element isn't within this window's 
	///       render tree.
	fn focus(&mut self, ele: &'a dyn crate::Element) {
		self.focus.map(|e| e.focus_change(false));
		self.focus = Some(ele);
		ele.focus_change(true);
	}
	/// Clear focus
	fn clear_focus(&mut self) {
		self.focus.map(|e| e.focus_change(false));
		self.focus = None;
	}

	/// Move to the specified location in the tab order (using the index passed to `taborder_add`)
	fn tabto(&mut self, idx: usize) {
		if let Ok(i) = self.taborder.binary_search_by(|v| ::std::cmp::Ord::cmp(&v.0, &idx))
		{
			self.taborder_pos = i;
			let e = self.taborder[i].1;
			self.focus(e);
		}
	}


	/// Manually request a redraw of the window
	fn rerender(&mut self) {
		// Size the window to something sane if not sized (or 0 sized)
		if self.surface.rect().dims() == ::geom::Dims::new(0,0) {
			self.set_dims(250, 150);
			self.set_pos(150, 100);
		}

		self.decorator.render( self.surface.slice(Rect::new_full()), self.needs_force_rerender );
		let subsurf = self.surface.slice(self.client_rect());
		self.root.render( subsurf, self.needs_force_rerender );
		self.surface.blit_to_win( &self.win );
		self.needs_force_rerender = false;
	}

	/// Obtain the states of all "modifier" keys
	fn get_modifiers(&self) -> &ModifierStates {
		&self.modifier_states
	}
}

impl<'a, D: Decorator> Window<'a, D>
{
	fn client_rect(&self) -> Rect<::geom::Px> {
		let (tl, br) = self.decorator.client_rect();
		let d = self.surface.rect();
		Rect::new(
			tl.w, tl.h,
			d.w - tl.w - br.w, d.h - tl.h - br.h
			)
	}
	fn update_surface_size(&mut self) {
		self.needs_force_rerender = true;
		self.surface.resize( self.win.get_dims(), self.background );
		let sub_dims = self.client_rect();
		self.root.resize( sub_dims.w.0, sub_dims.h.0 );
	}

	// Returns redraw status
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
			if ! self.client_rect().contains( Pos::new(x,y) ) {
				self.decorator.handle_event(ev, self);
				false
			}
			else {
				// TODO: Also send an event to the source element
				self.root.with_element_at_pos( Pos::new(x,y), self.surface.rect().dims(),
					&mut |ele, p| ele.handle_event(::InputEvent::MouseMove(p.x.0, p.y.0, dx, dy), self)
					)
			}
			},
		::InputEvent::MouseUp(x,y,btn) => {
			if ! self.client_rect().contains( Pos::new(x,y) ) {
				self.decorator.handle_event(ev, self);
				false
			}
			else {
				// TODO: Also send MouseUp to the element that received the MouseDown
				self.root.with_element_at_pos( Pos::new(x,y), self.surface.rect().dims(),
					&mut |ele, p| ele.handle_event( ::InputEvent::MouseUp(p.x.0,p.y.0, btn), self )
					)
			}
			},
		::InputEvent::MouseDown(x,y,btn) => {
			if ! self.client_rect().contains( Pos::new(x,y) ) {
				self.decorator.handle_event(ev, self);
				false
			}
			else {
				self.root.with_element_at_pos( Pos::new(x,y), self.surface.rect().dims(),
					&mut |ele, p| ele.handle_event( ::InputEvent::MouseDown(p.x.0, p.y.0, btn), self )
					)
			}
			},
		ev @ _ => {
			let ::decorator::EventHandled { capture, rerender } = self.decorator.handle_event(ev, self);
			if capture
			{
				rerender
			}
			else
			{
				match ev {
				::InputEvent::KeyDown(key) =>
					if let Some( (m,side) ) = Modifier::from_key(key)
					{
						self.modifier_states.set(m, side);
					},
				::InputEvent::KeyUp(key) =>
					if let Some( (m,side) ) = Modifier::from_key(key)
					{
						self.modifier_states.clear(m, side);
					},
				::InputEvent::KeyFire(key) => {
					for &mut ( (s_key, ref mods), ref mut fcn) in self.shortcuts.iter_mut() {
						if mods.check(&self.modifier_states) && key == s_key {
							fcn();
							return false || rerender;
						}
					}
					// - Single-key shortcuts
					//  TODO: Prevent meta-key shortcuts from firing if they were used as part of a multi-key shortcut
					for &mut (s_key, ref mut fcn) in self.shortcuts_0.iter_mut() {
						if key == s_key {
							fcn();
							return false || rerender;
						}
					}
					},
				_ => {},
				}
				
				if let Some(ele) = self.focus {
					ele.handle_event(ev, self) || rerender
				}
				else {
					false || rerender
				}
			}
			},
		}
	}
}

impl<'a, D: Decorator> ::r#async::WaitController for Window<'a, D>
{
	fn get_count(&self) -> usize {
		1
	}
	fn populate(&self, cb: &mut dyn FnMut(::syscalls::WaitItem)) {
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

#[repr(C)]
#[derive(Copy,Clone)]
pub enum Modifier {
	Shift = 0,
	Ctrl  = 1,
	Alt   = 2,
	Gui   = 3,
	Menu  = 4,
}
impl Modifier
{
	fn from_key(key: KeyCode) -> Option<(Modifier, bool)> {
		match key
		{
		KeyCode::LeftShift  => Some( (Modifier::Shift, false) ),
		KeyCode::RightShift => Some( (Modifier::Shift, true ) ),
		KeyCode::LeftCtrl  => Some( (Modifier::Ctrl, false) ),
		KeyCode::RightCtrl => Some( (Modifier::Ctrl, true ) ),
		KeyCode::LeftAlt  => Some( (Modifier::Alt, false) ),
		KeyCode::RightAlt => Some( (Modifier::Alt, true ) ),
		KeyCode::LeftGui  => Some( (Modifier::Gui, false) ),
		KeyCode::RightGui => Some( (Modifier::Gui, true ) ),
		KeyCode::Application  => Some( (Modifier::Menu, false) ),
		//KeyCode::RightMenu => Some( (Modifier::Menu, true ) ),
		_ => None,
		}
	}
}
#[derive(Default)]
pub struct ModifierStates(u16);
impl ModifierStates {
	fn mask(v: u16, m: Modifier) -> u16 {
		(v & 3) << ((m as u8) * 2)
	}
	pub fn set(&mut self, m: Modifier, right: bool) {
		self.0 |= Self::mask(if right { 2 } else { 1 }, m);
	}
	pub fn clear(&mut self, m: Modifier, right: bool) {
		self.0 &= !Self::mask(if right { 2 } else { 1 }, m);
	}
	pub fn test(&self, m: Modifier) -> bool {
		self.0 & Self::mask(3, m) != 0
	}
}
#[derive(Default,PartialEq)]
struct Modifiers(u8);
impl Modifiers {
	fn mask(m: Modifier) -> u8 {
		1 << (m as u8)
	}
	pub fn new(mods: &[Modifier]) -> Modifiers {
		let mut rv = 0;
		for &m in mods {
			rv |= Self::mask(m);
		}
		Modifiers(rv)
	}
	fn has(&self, m: Modifier) -> bool {
		self.0 & Self::mask(m) != 0
	}
	fn check(&self, states: &ModifierStates) -> bool {
		if self.has(Modifier::Shift) && ! states.test(Modifier::Shift) { return false; }
		if self.has(Modifier::Ctrl ) && ! states.test(Modifier::Ctrl ) { return false; }
		if self.has(Modifier::Alt  ) && ! states.test(Modifier::Alt  ) { return false; }
		if self.has(Modifier::Gui  ) && ! states.test(Modifier::Gui  ) { return false; }
		if self.has(Modifier::Menu ) && ! states.test(Modifier::Menu ) { return false; }
		true
	}
}

