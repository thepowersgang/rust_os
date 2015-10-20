// "Tifflin" Operating System - Window Toolkit
// - By John Hodge (thePowersGang)
//
// libwtk/menu.rs
//! Pop-up menu support

pub trait MenuTrait
{
	fn get_win(&self) -> &::syscalls::gui::Window;
}

pub struct Menu<I: MenuItems>
{
	window: ::syscalls::gui::Window,
	items: I,
}
impl<I: MenuItems> Menu<I> {
	pub fn new(debug_name: &str, items: I) -> Menu<I> {
		Menu {
			window: ::syscalls::gui::Window::new(debug_name).expect("TODO: Handle error in Menu::new()"),
			items: items,
		}
	}

	pub fn show(&self) {
		// TODO:
	}
}
impl<I> MenuTrait for Menu<I> {
	fn get_win(&self) -> &::syscalls::gui::Window {
		&self.window
	}
}

pub trait MenuItems {
}
impl<'a> MenuItems for Vec<AnyItem<'a>> {
}
//impl<I...: MenuItems> MenuItems for (I) {
//}
//impl<#N> MenuItems for [AnyItem; #N] {
//}

/// Hacky recursive macro 
macro_rules! impl_menu_items_tuple {
	( $s:ident : $n:ident = $v:expr ) => {
		impl<$n: MenuItem> MenuItems for ($n,) {
		}
	};
	( $s:ident : $n1:ident = $v1:expr, $($n:ident = $v:expr),+ ) => {
		impl<$n1: MenuItem, $($n: MenuItem),+> MenuItems for ($n1, $($n,)+) {
		}
		impl_menu_items_tuple!{ $s : $($n = $v),+ }
	};
}
//impl_menu_items_tuple! { self : I0 = self.0 }
//impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1 }
//impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1, I2 = self.2 }
//impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1, I2 = self.2, I3 = self.3 }
impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1, I2 = self.2, I3 = self.3, I4 = self.4 }

pub trait MenuItem {
	fn select(&self);
	fn render(&self, surf: ::surface::SurfaceView, hover: bool);
}


pub enum AnyItem<'a> {
	Spacer(Spacer),
	Label(Label),
	Entry(Entry<&'a Fn()>),
}
impl<'a> MenuItem for AnyItem<'a> {
	fn select(&self) {
	}
	fn render(&self, surf: ::surface::SurfaceView, hover: bool) {
	}
}

pub struct Spacer;

impl MenuItem for Spacer {
	fn select(&self) {
	}
	fn render(&self, surf: ::surface::SurfaceView, hover: bool) {
	}
}


pub struct Label {
	value: String,
}

impl MenuItem for Label {
	fn select(&self) {
	}
	fn render(&self, surf: ::surface::SurfaceView, hover: bool) {
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
	fn select(&self) {
	}
	fn render(&self, surf: ::surface::SurfaceView, hover: bool) {
	}
}

