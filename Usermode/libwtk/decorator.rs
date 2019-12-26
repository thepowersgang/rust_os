//
//
//
//! Window Decorations
use geom::{Rect,Dims,Px};
use surface::{SurfaceView, Colour};

/// Decorator interface
pub trait Decorator
{
	fn set_title<S: Into<String>>(&mut self, title: S);

	fn render(&self, surface: SurfaceView, full_redraw: bool);
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>);

	fn handle_event(&self, ev: ::InputEvent, win: &dyn crate::window::WindowTrait) -> EventHandled;
}
#[derive(Copy,Clone,Default)]
pub struct EventHandled
{
	pub capture: bool,
	pub rerender: bool,
}

impl Decorator for ()
{
	fn set_title<S: Into<String>>(&mut self, _title: S) {
	}

	fn render(&self, _surface: SurfaceView, _full_redraw: bool) {
	}
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>) {
		(Dims::new(0,0), Dims::new(0,0))
	}

	fn handle_event(&self, _ev: ::InputEvent, _win: &dyn crate::window::WindowTrait) -> EventHandled {
		Default::default()
	}
}

impl<D: Decorator> Decorator for Option<D>
{
	fn set_title<S: Into<String>>(&mut self, title: S) {
		self.as_mut().map(|x| x.set_title(title));
	}

	fn render(&self, surface: SurfaceView, full_redraw: bool) {
		match self
		{
		&Some(ref x) => x.render(surface, full_redraw),
		&None => {},
		}
	}
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>) {
		match self
		{
		&Some(ref x) => x.client_rect(),
		&None => (Dims::new(0,0), Dims::new(0,0)),
		}
	}

	fn handle_event(&self, ev: ::InputEvent, win: &dyn crate::window::WindowTrait) -> EventHandled {
		match self
		{
		&Some(ref x) => x.handle_event(ev, win),
		&None => Default::default(),
		}
	}
}

#[derive(Debug)]
enum MouseRegion
{
	BorderTopLeft,
	BorderTop,
	BorderTopRight,
	BorderLeft,
	BorderRight,
	BorderBottomLeft,
	BorderBottom,
	BorderBottomRight,
	ButtonClose,
}

//pub enum Buttons
//{
//	None,
//	Close,
//	CloseMin,
//	CloseMaxMin,
//}
/// Standard window decorations
#[derive(Default)]
pub struct Standard
{
	dirty: ::std::cell::Cell<bool>,
	title: String,
}
impl Standard
{
	pub fn new() -> Standard {
		Default::default()
	}
	fn win_template(&self) -> &Template<&[u32]> {
		const fn as_slice<T>(v: &[T]) -> &[T] { v } 
		static TEMPLATE: Template<&'static [u32]> = Template {
			w: 3, h: (2+16+2)+1+1,
			left: 1, top: (2+16+2),
			data: as_slice(&[
				// Top fixed region
				0xFFFFFF, 0xFFFFFF, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,

				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0x000000, 0xFFFFFF,

				0xFFFFFF, 0x000000, 0xFFFFFF,
				0xFFFFFF, 0xFFFFFF, 0xFFFFFF,

				// Middle variable
				0xFFFFFF, 0xFF_000000, 0xFFFFFF,

				// Bottom fixed
				0xFFFFFF, 0xFFFFFF, 0xFFFFFF,
				]),
			};
		&TEMPLATE
	}
	fn buttton_template(&self) -> &Template<[u32]> {
		static TEMPLATE: Template<[u32; 5*5]> = Template {
			w: 5, h: 5,
			left: 2, top: 2,
			data: [
				// Top fixed region
				0xFFFFFF, 0xFFFFFF,  0xFFFFFF,  0xFFFFFF, 0xFFFFFF,
				0xFFFFFF, 0x000000,  0x000000,  0x000000, 0xFFFFFF,

				// Middle variable
				0xFFFFFF, 0x000000,  0xFF_000000,  0x000000, 0xFFFFFF,

				// Bottom fixed
				0xFFFFFF, 0x000000,  0x000000,  0x000000, 0xFFFFFF,
				0xFFFFFF, 0xFFFFFF,  0xFFFFFF,  0xFFFFFF, 0xFFFFFF,
				],
			};
		&TEMPLATE
	}
	fn button_width(&self) -> u32 { 16 }
	fn button_height(&self) -> u32 { 16 }

	fn titlebar_left(&self) -> u32 { 2 }
	fn titlebar_top(&self) -> u32 { 2 }
	fn titlebar_right(&self) -> u32 { 2 }
	fn titlebar_height(&self) -> u32 { 16 }

	fn render_button_exit(&self, surface: SurfaceView) {
		self.buttton_template().render( surface.clone() );
		// Draw an X in the middle
		// TODO: ^
	}

	fn mouse_region(&self, x: u32, y: u32, w: u32, h: u32) -> Option<MouseRegion> {
		if y < self.win_template().bottom() { //self.win_template().top() {
			if x < self.win_template().left() {
				Some(MouseRegion::BorderTopLeft)
			}
			else if x < w - self.win_template().right() {
				Some(MouseRegion::BorderTop)
			}
			else {
				Some(MouseRegion::BorderTopRight)
			}
		}
		else if y < self.titlebar_height() {
			if x < self.win_template().left() {
				Some(MouseRegion::BorderLeft)
			}
			else if x < w - self.win_template().right() {
				let right = w - self.win_template().right();
				// TODO: Buttons/move
				if x > right - self.button_width() {
					Some(MouseRegion::ButtonClose)
				}
				else {
					None
				}
			}
			else {
				Some(MouseRegion::BorderRight)
			}
		}
		else if y < h - self.win_template().bottom() {
			if x < self.win_template().left() {
				Some(MouseRegion::BorderLeft)
			}
			else if x < w - self.win_template().right() {
				// Client region
				None
			}
			else {
				Some(MouseRegion::BorderRight)
			}
		}
		else {
			if x < self.win_template().left() {
				Some(MouseRegion::BorderBottomLeft)
			}
			else if x < w - self.win_template().right() {
				Some(MouseRegion::BorderBottom)
			}
			else {
				Some(MouseRegion::BorderBottomRight)
			}
		}
	}
}
impl Decorator for Standard
{
	fn set_title<S: Into<String>>(&mut self, title: S) {
		self.dirty.set(true);
		self.title = title.into();
	}

	fn render(&self, surface: SurfaceView, full_redraw: bool) {
		let title_dirty = self.dirty.get(); self.dirty.set(false);

		if title_dirty || full_redraw
		{
			// If not doing a full redraw, only re-blit the title bar
			if ! full_redraw {
				self.win_template().render_lines(surface.clone(), self.win_template().top());
			}
			else {
				self.win_template().render(surface.clone());
			}

			// Draw buttons
			// - Exit (always present)
			let mut right_x = surface.rect().x2().0 - self.titlebar_right();
			right_x -= self.button_width();
			self.render_button_exit( surface.slice( Rect::new( right_x, self.titlebar_top(), self.button_width(), self.button_height() ) ) );

			// - Maximise (optional)
			//if self.button_mode.has_maximise() {
			//	right_x -= theme.button_width();
			//	self.render_button_maximise( surface.slice( Rect::new( right_x, self.titlebar_top(), self.button_width(), self.button_height() ) ) );
			//}

			// - Minimise (optional)
			//if self.button_mode.has_minimise() {
			//	right_x -= theme.button_width();
			//	self.render_button_minimise( surface.slice( Rect::new( right_x, self.titlebar_top(), self.button_width(), self.button_height() ) ) );
			//}

			// Draw icon and title
			// - Icon
			//let mut left_x = theme.inner_left();
			//if let Some(ref icon) = self.icon
			//{
			//	self.icon.render( surface.slice( Rect::new(left_x, theme.titlebar_top(), 16, 16) ) );
			//	left_x += 16 + 2;
			//}

			// Draw title (last, to properly clip text)
			surface.draw_text( Rect::new_pts( self.titlebar_left(), self.titlebar_top(),  right_x, self.titlebar_height() + self.titlebar_top() ), self.title.chars(), Colour::from_argb32(0xFFFFFF) );
		}
	}
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>) {
		(
			Dims::new(self.win_template().left() , self.win_template().top()),
			Dims::new(self.win_template().right(), self.win_template().bottom())
			)
	}

	fn handle_event(&self, ev: ::InputEvent, win: &dyn crate::window::WindowTrait) -> EventHandled {
		let modifiers = win.get_modifiers();
		let (w, h) = win.get_full_dims();
		match ev
		{
		// Alt-F4
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::F4) => {
			if modifiers.test(::window::Modifier::Alt) {
				//self.exit_handler();	// TODO: How can I cleanly do this
				::syscalls::threads::exit(0);
				// ^ Diverges
			}
			Default::default()
			},
		// Alt-Space
		::InputEvent::KeyUp(::syscalls::gui::KeyCode::Space) => {
			if modifiers.test(::window::Modifier::Alt) {
				// TODO: Show titlebar menu
			}
			Default::default()
			},
		
		// Click
		::InputEvent::MouseUp(x,y,0) =>
			match self.mouse_region(x,y, w,h)
			{
			Some(MouseRegion::ButtonClose) => ::syscalls::threads::exit(0),
			Some(r) => {	// TODO: Other regions
				kernel_log!("TODO: Region {:?} wxh={}x{}", r, w, h);
				Default::default()
				},
			None => Default::default(),
			},
		_ => Default::default(),
		}
	}
}

struct Template<T: ?Sized + AsRef<[u32]>> {
	/// Total width
	w: u32,
	/// Total width
	h: u32,
	/// Width of the lefthand fixed region
	left: u32,
	/// Height of the top fixed region
	top: u32,
	/// Image data
	data: T,
}
impl<T: ?Sized + AsRef<[u32]>> Template<T>
{
	pub fn render(&self, surface: SurfaceView) {
		self.render_lines(surface, !0)
	}
	pub fn render_lines(&self, surface: SurfaceView, nlines: u32)
	{
		if surface.height() < self.fixed_height() {
			return ;
		}
		let dst_bottom = (surface.height() - self.bottom()) as usize;
		surface.foreach_scanlines(Rect::new(0,0,!0,nlines), |row, sl|
			if row < self.top as usize {
				self.render_line( row, sl );
			}
			else if row < dst_bottom {
				self.render_line( self.top as usize, sl );
			}
			else {
				let i = row - dst_bottom;
				self.render_line( self.top as usize + 1 + i, sl );
			}
			);
	}

	fn render_line(&self, sline: usize, dst: &mut [u32]) {
		if dst.len() <= self.fixed_width() as usize {
			return ;
		}
		let src = &self.data.as_ref()[self.w as usize * sline .. ][ .. self.w as usize];
		let dst_w = dst.len();
		let dst_right = dst_w - self.right() as usize;

		for col in 0 .. self.left as usize {
			dst[col] = src[col];
		}

		// if mid_c's top byte is 0xFF, it's fully transparent
		let mid_c = src[self.left as usize];
		if mid_c >> 24 != 0xFF
		{
			for col in self.left as usize .. dst_right {
				dst[col] = mid_c;
			}
		}

		for i in 0 .. self.right() as usize {
			dst[dst_right + i] = src[self.left as usize + 1 + i];
		}
	}
	pub fn fixed_width(&self) -> u32 {
		self.w - 1
	}
	pub fn left(&self) -> u32 {
		self.left
	}
	pub fn right(&self) -> u32 {
		self.w - self.left - 1
	}
	pub fn fixed_height(&self) -> u32 {
		self.h - 1
	}
	pub fn top(&self) -> u32 {
		self.top
	}
	pub fn bottom(&self) -> u32 {
		self.h - self.top - 1
	}
}

