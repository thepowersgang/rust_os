// "Tifflin" Operating System - Window Toolkit
// - By John Hodge (thePowersGang)
//
// libwtk/menu.rs
//! Pop-up menu support
use geom::Rect;
use surface::Colour;

pub struct Menu<I: MenuItems>
{
	window: ::syscalls::gui::Window,
	hilight: ::std::cell::RefCell<usize>,
	buffer: ::surface::Surface,
	items: I,
}
impl<I: MenuItems> Menu<I> {
	/// Create a new popup menu
	pub fn new(debug_name: &str, items: I) -> Menu<I> {
		let dims = items.total_dims();
		let dims = ::syscalls::gui::Dims { w: dims.0, h: dims.1 };
		Menu {
			window: {
				let w = ::syscalls::gui::Window::new(debug_name).expect("TODO: Handle error in Menu::new()");
				w.set_dims( dims );
				w
				},
			hilight: ::std::cell::RefCell::new(0),
			buffer: {
				let mut s = ::surface::Surface::default();
				s.resize( dims, Colour::theme_text_bg() );
				s
				},
			items: items,
		}
	}

	pub fn show(&self) {
		kernel_log!("Showing menu");
		self.render();
		self.buffer.blit_to_win(&self.window);
		self.window.show();
	}
	
	// NOTE: When this menu loses focus, it should hide itself
	// - UNLESS it's just opened a sub-menu
	
	pub fn render(&self) {
		let bh = &self.buffer;
		self.items.render( *self.hilight.borrow(), bh.slice(Rect::new(0,0, !0,!0)) );
	}

	fn handle_event(&self, ev: ::InputEvent) -> bool {
		use syscalls::gui::KeyCode;
		match ev
		{
		::InputEvent::KeyUp(KeyCode::UpArrow) => {
			let mut hl = self.hilight.borrow_mut();
			if *hl > 0 {
				*hl += 1;
				true
			}
			else {
				false
			}
			},
		::InputEvent::KeyUp(KeyCode::DownArrow) => {
			let mut hl = self.hilight.borrow_mut();
			if *hl < self.items.count()-1 {
				*hl += 1;
				true
			}
			else {
				false
			}
			},
		::InputEvent::KeyUp(KeyCode::Return) => {
			// TODO: Close menu
			self.items.select( *self.hilight.borrow() );
			true
			},
		::InputEvent::KeyUp(KeyCode::Esc) => {
			// TODO: Close menu
			false
			},
		// Mouse events need to be dispatched correctly
		::InputEvent::MouseMove(_x,y,_dx,_dy) => {
			let idx = self.items.get_idx_at_y(y);
			*self.hilight.borrow_mut() = idx;
			true
			},
		::InputEvent::MouseUp(_x,y,0) => {
			let idx = self.items.get_idx_at_y(y);
			let mut hl = self.hilight.borrow_mut();
			*hl = idx;
			self.items.select(idx);
			true
			},
		//::InputEvent::MouseDown(x,y,btn) => false,
		_ => false,
		}
	}
}

pub struct WaitWrapper<'a, I: 'a + MenuItems>(&'a Menu<I>);

impl<'a, I: 'a + MenuItems> ::async::WaitController for WaitWrapper<'a, I>
{
	fn get_count(&self) -> usize {
		1
	}
	fn populate(&self, cb: &mut FnMut(::syscalls::WaitItem)) {
		use syscalls::Object;
		cb( self.0.window.get_wait( ::syscalls::gui::WindowWaits::new().input() ) )
	}
	fn handle(&mut self, events: &[::syscalls::WaitItem]) {
		use syscalls::Object;
		let mut redraw = false;
		if self.0.window.check_wait(&events[0]).has_input()
		{
			while let Some(ev) = self.0.window.pop_event()
			{
				redraw |= self.0.handle_event(ev);
			}
		}

		if redraw {
			self.0.render();
			self.0.window.redraw();
		}
	}
}

pub trait MenuItems {
	fn count(&self) -> usize;
	fn get_idx_at_y(&self, y: u32) -> usize;
	fn total_dims(&self) -> (u32,u32);
	fn select(&self, index: usize);
	fn render(&self, focus: usize, surf: ::surface::SurfaceView);
}
impl<'a> MenuItems for Vec<AnyItem<'a>> {
	fn count(&self) -> usize {
		self.len()
	}
	fn get_idx_at_y(&self, y: u32) -> usize {
		let mut ofs = 0;
		for (i,e) in self.iter().enumerate() {
			let h = e.dims().height().0;
			if y < ofs + h {
				return i;
			}
			ofs += h;
		}
		return self.len();
	}
	fn total_dims(&self) -> (u32,u32) {
		let mut rv = (0,0);
		for e in self.iter() {
			let d = e.dims();
			rv.0 += ::std::cmp::max(d.width().0, rv.0);
			rv.1 += d.height().0;
		}
		rv
	}
	fn select(&self, index: usize) {
		if index < self.len() {
			self[index].select()
		}
		else {
		}
	}
	fn render(&self, focus: usize, surf: ::surface::SurfaceView) {
		let mut y = 0;
		for (i,e) in self.iter().enumerate()
		{
			let h = e.dims().height().0;
			e.render(surf.slice(::geom::Rect::new( 0,y, !0,h)), i == focus);
			y += h;
		}
	}
}
// NOTE: This code is just taking a punt at how vardic generics would look
// - "for e in self..." iterates over each I in (I...)
//impl<I...: MenuItem> MenuItems for (I...) {
//	fn total_dims(&self) -> (u32,u32) {
//		let (mut w, mut h) = (0,0)
//		... {
//			let d = (*self).dims()
//			w = ::std::cmp::max( d.width().0, w );
//			h += d.height().0;
//		}
//		for e in self... {
//			let d = e.dims();
//			w = ::std::cmp::max( d.width().0, w );
//			h += d.height().0;
//		}
//		(w, h)
//	}
//	fn select(&self, index: usize) {
//		let mut i = 0;
//		for e in self... {
//			if index == i { return e.select(); }
//			i += 1;
//		}
//	}
//	fn render(&self, focus: usize, surf: ::surface::SurfaceView) {
//		let mut y = h;
//		let mut i = 0;
//		for e in self... {
//			let h = e.dims().height().0;
//			e.render(surf.slice(::geom::Rect::new(0,y, !0,h)), i == focus);
//			y += h;
//			i += 1;
//		}
//	}
//}
//impl<#N> MenuItems for [AnyItem; #N] {
//	fn total_dims(&self) -> (u32,u32) {
//		let (mut w, mut h) = (0,0)
//		for e in self {
//			let d = e.dims();
//			w = ::std::cmp::max( d.width().0, w );
//			h += d.height().0;
//		}
//		(w, h)
//	}
//}

/// Hacky recursive macro 
macro_rules! impl_menu_items_tuple {
	( $s:ident : $n:ident = $v:expr ) => {
		impl<$n: MenuItem> MenuItems for ($n,) {
			fn count(&$s) -> usize {
				1
			}
			fn get_idx_at_y(&$s, y: u32) -> usize {
				if y < $v.dims().height().0 {
					0
				}
				else {
					1
				}
			}
			fn total_dims(&$s) -> (u32,u32) {
				let d = $v.dims();
				(d.width().0, d.height().0)
			}
			fn select(&$s, index: usize) {
				if index == 0 {
					$v.select();
				}
			}
			fn render(&$s, focus: usize, surf: ::surface::SurfaceView) {
				$v.render(surf, focus == 0);
			}
		}
	};
	( $s:ident : $n1:ident = $v1:expr, $($n:ident = $v:expr),+ ) => {
		impl<$n1: MenuItem, $($n: MenuItem),+> MenuItems for ($n1, $($n,)+) {
			fn count(&$s) -> usize {
				1 $(+ {let _ = $v; 1})*
			}
			fn get_idx_at_y(&$s, y: u32) -> usize {
				let h = $v1.dims().height().0;
				if y < h {
					return 0;
				}
				let mut ofs = h;
				let mut i = 1;
				$(
				let h = $v.dims().height().0;
				if y < ofs + h {
					return i;
				}
				ofs += h;
				i += 1;
				)*
				let _ = ofs;
				// Return self.count()
				i
			}
			fn total_dims(&$s) -> (u32,u32) {
				let d = $v1.dims();
				let mut rv = (d.width().0,d.height().0);
				$(
				let d = $v.dims();
				rv.0 += ::std::cmp::max(d.width().0, rv.0);
				rv.1 += d.height().0;
				)*
				rv
			}
			fn select(&$s, index: usize) {
				let mut i = 0 $(+ {let _ = $v; 1})*;
				if index == i { return $v1.select(); }
				$(
				i -= 1;
				if index == i { return $v.select(); }
				)*
			}
			fn render(&$s, focus: usize, surf: ::surface::SurfaceView) {
				let mut y = 0 $( + $v.dims().height().0)*;
				let mut i = 0 $( + {let _ = $v; 1})*;

				let h = $v1.dims().height().0;
				$v1.render(surf.slice(::geom::Rect::new(0,y, !0,h)), i == focus);
				$(
				let h = $v.dims().height().0;
				y -= h;
				i -= 1;
				$v.render(surf.slice(::geom::Rect::new(0,y, !0,h)), i == focus);
				)*
			}
		}
		impl_menu_items_tuple!{ $s : $($n = $v),+ }
	};
}
// Only need one invocation, tuple args must be in reverse order.
impl_menu_items_tuple! { self : I4 = self.4, I3 = self.3, I2 = self.2, I1 = self.1, I0 = self.0 }

pub trait MenuItem {
	fn dims(&self) -> ::geom::Rect<::geom::Px>;
	fn select(&self);
	fn render(&self, surf: ::surface::SurfaceView, hover: bool);
}


pub enum AnyItem<'a> {
	Spacer(Spacer),
	Label(Label),
	Entry(Entry<&'a Fn()>),
}
impl<'a> MenuItem for AnyItem<'a> {
	fn dims(&self) -> ::geom::Rect<::geom::Px> {
		match self
		{
		&AnyItem::Spacer(ref e) => e.dims(),
		&AnyItem::Label(ref e)  => e.dims(),
		&AnyItem::Entry(ref e)  => e.dims(),
		}
	}
	fn select(&self) {
		match self
		{
		&AnyItem::Spacer(ref e) => e.select(),
		&AnyItem::Label(ref e)  => e.select(),
		&AnyItem::Entry(ref e)  => e.select(),
		}
	}
	fn render(&self, surf: ::surface::SurfaceView, hover: bool) {
		match self
		{
		&AnyItem::Spacer(ref e) => e.render(surf, hover),
		&AnyItem::Label(ref e)  => e.render(surf, hover),
		&AnyItem::Entry(ref e)  => e.render(surf, hover),
		}
	}
}

pub struct Spacer;

impl MenuItem for Spacer {
	fn dims(&self) -> ::geom::Rect<::geom::Px> {
		::geom::Rect::new(0,0, 0,3)
	}
	fn select(&self) {
		// Do nothing
	}
	fn render(&self, surf: ::surface::SurfaceView, _hover: bool) {
		surf.fill_rect(Rect::new(1,1, !0,!0), Colour::theme_text_alt());
	}
}


pub struct Label {
	value: String,
}

impl MenuItem for Label {
	fn dims(&self) -> ::geom::Rect<::geom::Px> {
		::geom::Rect::new(0,0, 0,18)
	}
	fn select(&self) {
		// Do nothing
	}
	fn render(&self, surf: ::surface::SurfaceView, _hover: bool) {
		surf.draw_text(Rect::new(1,1,!0,!0), self.value.chars(), Colour::theme_text());
	}
}


pub struct Entry<A>
where
	A: Fn()
{
	label: String,
	accel_ofs: usize,
	altlabel: String,

	label_width: u32,
	altlabel_width: u32,
	
	action: A,
}

impl<A: Fn()> Entry<A> {
	pub fn new<Lab: Into<String>, Alt: Into<String>>(label: Lab, accel: usize, alt: Alt, action: A) -> Entry<A> {
		let label = label.into();
		let altlabel = alt.into();
		Entry {
			label_width: label.len() as u32 * 8,
			altlabel_width: altlabel.len() as u32 * 8,

			label: label,
			accel_ofs: accel,
			altlabel: altlabel,
			action: action,
			}
	}
}
impl<A: Fn()> MenuItem for Entry<A> {
	fn dims(&self) -> ::geom::Rect<::geom::Px> {
		const MARGIN_WIDTH: u32 = 1;
		const LABEL_GAP: u32 = 5;
		::geom::Rect::new(0,0, MARGIN_WIDTH*2 + self.label_width + LABEL_GAP + self.altlabel_width, MARGIN_WIDTH*2 + 16)
	}
	fn select(&self) {
		(self.action)()
	}
	fn render(&self, surf: ::surface::SurfaceView, hover: bool) {
		let fg = if hover {
				Colour::theme_text_alt()
			}
			else {
				Colour::theme_text()
			};
		surf.draw_text(Rect::new(1,1,!0,!0), self.label.chars(), fg);

		let total_w = surf.width();
		surf.draw_text(Rect::new(total_w - self.altlabel_width - 1, 1, !0,!0), self.altlabel.chars(), fg);
	}
}

