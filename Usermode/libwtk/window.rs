
use geom::Rect;

pub struct Window<'a>
{
	win: ::syscalls::gui::Window,
	surface: ::surface::Surface,
	root: &'a ::Element,

	//input_state: KeyboardState,
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
			root: ele
		}
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
		0
	}
}

