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
	hilight: usize,
	buffer: ::surface::Surface,
	items: I,
}
impl<I: MenuItems> Menu<I> {
	pub fn new(debug_name: &str, items: I) -> Menu<I> {
		let dims = items.total_dims();
		let dims = ::syscalls::gui::Dims { w: dims.0, h: dims.1 };
		Menu {
			window: {
				let mut w = ::syscalls::gui::Window::new(debug_name).expect("TODO: Handle error in Menu::new()");
				w.set_dims( dims );
				w
				},
			hilight: 0,
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
		self.buffer.blit_to_win(&self.window);
		self.window.show();
	}
	
	// NOTE: When this menu loses focus, it should hide itself
	// - UNLESS it's just opened a sub-menu
	
	pub fn render(&self) {
		let bh = &self.buffer;
		self.items.render(self.hilight, bh.slice(Rect::new(0,0, !0,!0)));
	}
}

pub trait MenuItems {
	fn total_dims(&self) -> (u32,u32);
	fn select(&self, index: usize);
	fn render(&self, focus: usize, surf: ::surface::SurfaceView);
}
impl<'a> MenuItems for Vec<AnyItem<'a>> {
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
//impl<I...: MenuItems> MenuItems for (I) {
//}
//impl<#N> MenuItems for [AnyItem; #N] {
//}

/// Hacky recursive macro 
macro_rules! impl_menu_items_tuple {
	( $s:ident : $n:ident = $v:expr ) => {
		impl<$n: MenuItem> MenuItems for ($n,) {
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
				let mut i = 0;
				if index == i { return $v1.select(); }
				i += 1;
				$(
				if index == i { return $v.select(); }
				)*
				let _ = i;
			}
			fn render(&$s, focus: usize, surf: ::surface::SurfaceView) {
				let h = $v1.dims().height().0;
				$v1.render(surf.slice(::geom::Rect::new(0,0, !0,h)), focus == 0);
				let mut y = h;
				let mut i = 1;
				$(
				let h = $v.dims().height().0;
				$v.render(surf.slice(::geom::Rect::new(0,y, !0,h)), i == focus);
				y += h;
				i += 1;
				)*
				let _ = (y, i);
			}
		}
		impl_menu_items_tuple!{ $s : $($n = $v),+ }
	};
}
//impl_menu_items_tuple! { self : I0 = self.0 }
//impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1 }
//impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1, I2 = self.2 }
//impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1, I2 = self.2, I3 = self.3 }
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
	
	action: A,
}

impl<A: Fn()> Entry<A> {
	pub fn new<Lab: Into<String>, Alt: Into<String>>(label: Lab, accel: usize, alt: Alt, action: A) -> Entry<A> {
		Entry {
			label: label.into(),
			accel_ofs: accel,
			altlabel: alt.into(),
			action: action,
			}
	}
}
impl<A: Fn()> MenuItem for Entry<A> {
	fn dims(&self) -> ::geom::Rect<::geom::Px> {
		::geom::Rect::new(0,0, 0,18)
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

		let (alw,_) = surf.size_text(self.altlabel.chars());
		let total_w = surf.width();
		surf.draw_text(Rect::new(total_w - alw as u32 - 1, 1, !0,!0), self.altlabel.chars(), fg);
	}
}

