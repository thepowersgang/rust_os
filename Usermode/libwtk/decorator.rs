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

	fn render(&self, surface: SurfaceView);
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>);

	fn handle_event(&self, ev: ::InputEvent) -> bool;
}

impl Decorator for ()
{
	fn set_title<S: Into<String>>(&mut self, _title: S) {
	}

	fn render(&self, _surface: SurfaceView) {
	}
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>) {
		(Dims::new(0,0), Dims::new(0,0))
	}

	fn handle_event(&self, ev: ::InputEvent) -> bool {
		false
	}
}

impl<D: Decorator> Decorator for Option<D>
{
	fn set_title<S: Into<String>>(&mut self, title: S) {
		self.as_mut().map(|x| x.set_title(title));
	}

	fn render(&self, surface: SurfaceView) {
		self.as_ref().map(|x| x.render(surface));
	}
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>) {
		match self
		{
		&Some(ref x) => x.client_rect(),
		&None => (Dims::new(0,0), Dims::new(0,0)),
		}
	}

	fn handle_event(&self, ev: ::InputEvent) -> bool {
		match self
		{
		&Some(ref x) => x.handle_event(ev),
		&None => false,
		}
	}
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
	title: String,
}
impl Decorator for Standard
{
	fn set_title<S: Into<String>>(&mut self, title: S) {
		self.title = title.into();
	}

	fn render(&self, surface: SurfaceView) {
		let theme = StandardTheme;
		theme.win_template().render(surface.clone());
		//let mut left_x = theme.inner_left();
		//self.icon.render( surface.slice( Rect::new(left_x, theme.titlebar_top(), 16, 16) ) );
		//left_x += 16 + 2;
		surface.draw_text( Rect::new( theme.titlebar_left(), theme.titlebar_top(),  !0, !0 ), self.title.chars(), Colour::theme_text() );

		//StandardTheme.buttton_template().render(
		//	surface.slice( Rect::new( surface.width() - theme.titlebar_right() - theme.button_width(), surface.titlebar_top(), theme.button_width(), theme.button_height() ) )
		//	);
	}
	fn client_rect(&self) -> (Dims<Px>,Dims<Px>) {
		let theme = StandardTheme;
		( Dims::new(theme.client_left(), theme.client_top()), Dims::new(theme.client_right(), theme.client_bottom()) )
	}

	fn handle_event(&self, ev: ::InputEvent) -> bool {
		match ev
		{
		_ => false,
		}
	}
}


struct StandardTheme;
impl StandardTheme
{
	fn win_template(&self) -> &Template<&[u32]> {
		const fn as_slice<T>(v: &[T]) -> &[T] { v } 
		static TEMPLATE: Template<&'static [u32]> = Template {
			w: 3, h: (1+16+1)+1+1,
			left: 1, top: (1+16+1),
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
				0xFFFFFF, 0xFFFFFF, 0xFFFFFF,

				// Middle variable
				0xFFFFFF, 0xFF_000000, 0xFFFFFF,

				// Bottom fixed
				0xFFFFFF, 0xFFFFFF, 0xFFFFFF,
				]),
			};
		&TEMPLATE
	}
	//fn buttton_template(&self) -> &Template<[u32]> {
	//	static TEMPLATE: Template<[u32; 5*5]> = Template {
	//		w: 5, h: 5,
	//		left: 2, top: 2,
	//		data: [
	//			// Top fixed region
	//			0xFFFFFF, 0xFFFFFF,  0xFFFFFF,  0xFFFFFF, 0xFFFFFF,
	//			0xFFFFFF, 0x000000,  0x000000,  0x000000, 0xFFFFFF,

	//			// Middle variable
	//			0xFFFFFF, 0x000000,  0xFF_000000,  0x000000, 0xFFFFFF,

	//			// Bottom fixed
	//			0xFFFFFF, 0x000000,  0x000000,  0x000000, 0xFFFFFF,
	//			0xFFFFFF, 0xFFFFFF,  0xFFFFFF,  0xFFFFFF, 0xFFFFFF,
	//			],
	//		};
	//	&TEMPLATE
	//}

	fn titlebar_left(&self) -> u32 { 2 }
	fn titlebar_top(&self) -> u32 { 2 }
	//fn titlebar_right(&self) -> u32 { 2 }
	//fn titlebar_bottom(&self) -> u32 { 2 }

	fn client_left (&self) -> u32 { self.win_template().left() }
	fn client_right(&self) -> u32 { self.win_template().right() }
	fn client_top   (&self) -> u32 { self.win_template().top() }
	fn client_bottom(&self) -> u32 { self.win_template().bottom() }
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
impl<T: AsRef<[u32]>> Template<T>
{
	pub fn render(&self, surface: SurfaceView)
	{
		if surface.height() < self.fixed_height() {
			return ;
		}
		let dst_bottom = (surface.height() - self.bottom()) as usize;
		surface.foreach_scanlines(Rect::new(0,0,!0,!0), |row, sl|
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

