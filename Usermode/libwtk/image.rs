//
//
//
use geom::{Rect,Px};
use surface::Colour;

/// Static image wrapper
pub struct Image<T: Buffer>
{
	has_changed: ::std::cell::Cell<bool>,
	data: T,
}

impl<T: Buffer> Image<T>
{
	pub fn new(i: T) -> Image<T> {
		Image {
			has_changed: ::std::cell::Cell::new(true),
			data: i,
		}
	}

	pub fn force_redraw(&self) {
		self.has_changed.set(true); 
	}
}

impl<T: Buffer> ::Element for Image<T>
{
	fn focus_change(&self, _have: bool) {
		// Don't care
	}

	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool {
		// Don't care
		false
	}

	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		if force || self.has_changed.get() {
			self.data.render(surface);
			self.has_changed.set(false);
		}
	}
}


pub trait Buffer
{
	fn dims_px(&self) -> Rect<Px>;
	//fn dims_phys(&self) -> Rect<::geom::Mm>;
	fn render(&self, buf: ::surface::SurfaceView);
}

impl Buffer for ::surface::Colour {
	fn dims_px(&self) -> Rect<Px> {
		Rect::new(0,0,0,0)
	}
	fn render(&self, buf: ::surface::SurfaceView) {
		buf.fill_rect(buf.rect(), *self);
	}
}

/// Raster two-colour image with alpha
pub struct RasterMonoA
{
	bg: ::surface::Colour,
	fg: ::surface::Colour,
	width: usize,
	data: Vec<bool>,	// BitVec
	alpha: Vec<u8>,
}
impl RasterMonoA
{
	pub fn new<P: AsRef<::std::fs::Path>>(path: P) -> RasterMonoA {
		let path = path.as_ref();
		todo!("RasterMonoA::new() path = {:?}", path);
	}
}
impl Buffer for RasterMonoA {
	fn dims_px(&self) -> Rect<Px> {
		Rect::new(0,0,  self.width as u32, (self.data.len() / self.width) as u32)
	}
	fn render(&self, buf: ::surface::SurfaceView) {
		// - Alpha defaults to zero if the alpha vec is empty
		let mut buf_rows = Iterator::zip( self.data.chunks(self.width), self.alpha.chunks(self.width).chain(::std::iter::repeat(&[][..])) );
		buf.foreach_scanlines(self.dims_px(), |_row, line| {
			let (bitmap, alpha) = buf_rows.next().unwrap();
			for (d, (bm, a)) in Iterator::zip( line.iter_mut(), Iterator::zip( bitmap.iter(), alpha.iter().cloned().chain(::std::iter::repeat(0)) ) )
			{
				let c = if *bm { self.fg } else { self.bg };
				*d = Colour::blend_alpha( Colour::from_argb32(*d), c, a ).as_argb32();
			}
			});
	}
}

