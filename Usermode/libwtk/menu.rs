// "Tifflin" Operating System - Window Toolkit
// - By John Hodge (thePowersGang)
//
// libwtk/menu.rs
//! Pop-up menu support

pub struct Menu<I: MenuItems>
{
	window: ::syscalls::gui::Window,
	items: I,
}

pub trait MenuItems {
}
impl MenuItems for Vec<AnyItem> {
}
//impl<I...: MenuItems> MenuItems for (I) {
//}
//impl<#N> MenuItems for [AnyItem; #N] {
//}
macro_rules! impl_menu_items_tuple {
	( $s:ident : $($n:ident = $v:expr),* ) => {
		impl<$($n: MenuItem),*> MenuItems for ($($n,)*) {
		}
	};
}
impl_menu_items_tuple! { self : I0 = self.0 }
impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1 }
impl_menu_items_tuple! { self : I0 = self.0, I1 = self.1, I2 = self.2 }

pub trait MenuItem {
	fn select(&self);
	fn render(&self, surf: ::surface::SurfaceView, hover: bool);
}

pub enum AnyItem {
	Spacer(Spacer),
	Label(Label),
	Entry(Entry<Box<Fn()>>),
}
impl MenuItem for AnyItem {
}

pub struct Spacer;

impl MenuItem for Spacer {
}

struct Label {
	value: String,
}

struct Entry<A>
where
	A: Fn()
{
	label: String,
	accel_ofs: usize,
	altlabel: String,
	
	action: A,
}

