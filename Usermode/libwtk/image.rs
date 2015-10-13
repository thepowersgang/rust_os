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
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::Element, (u32,u32)) {
		(self, (0,0))
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

#[derive(Debug)]
pub enum LoadError {
	Io( ::std::io::Error ),
	Malformed,
}
impl_conv! {
	From<::std::io::Error>(v) for LoadError {{
		LoadError::Io(v)
	}}
	From<::byteorder::Error>(v) for LoadError {{
		match v
		{
		::byteorder::Error::Io(v) => LoadError::Io(v),
		::byteorder::Error::UnexpectedEOF => LoadError::Malformed,
		}
	}}
}

/// Raster single-colour image with alpha
pub struct RasterMonoA
{
	fg: ::surface::Colour,
	width: usize,
	alpha: Vec<u8>,
}
impl RasterMonoA
{
	pub fn new<P: AsRef<::std::fs::Path>>(path: P, fg: ::surface::Colour) -> Result<RasterMonoA,LoadError> {
		use ::byteorder::{LittleEndian,ReadBytesExt};
		let path = path.as_ref();
		let mut file = try!( ::std::fs::File::open(path) );
		let w = try!( file.read_u16::<LittleEndian>() ) as usize;
		let h = try!( file.read_u16::<LittleEndian>() ) as usize;
		kernel_log!("(w,h) = ({},{})", w, h);
		let mut alpha = Vec::with_capacity(w * h);
		for _ in 0 .. w * h
		{
			let v = try!( file.read_u8() );
			alpha.push( v );
		}
		Ok(RasterMonoA {
			fg: fg,
			width: w,
			alpha: alpha,
			})
	}
}
impl Buffer for RasterMonoA {
	fn dims_px(&self) -> Rect<Px> {
		Rect::new(0,0,  self.width as u32, (self.alpha.len() / self.width) as u32)
	}
	fn render(&self, buf: ::surface::SurfaceView) {
		// - Alpha defaults to zero if the alpha vec is empty
		let mut buf_rows = self.alpha.chunks(self.width);
		buf.foreach_scanlines(self.dims_px(), |_row, line| {
			let alpha = buf_rows.next().unwrap();
			for (d, a) in Iterator::zip( line.iter_mut(), alpha.iter().cloned() )
			{
				*d = Colour::blend_alpha( Colour::from_argb32(*d), self.fg, 255 - a ).as_argb32();
			}
			});
	}
}

/// Raster two-colour image with alpha
pub struct RasterBiA
{
	bg: ::surface::Colour,
	fg: ::surface::Colour,
	width: usize,
	data: Vec<bool>,	// TODO: Use BitVec or similar
	alpha: Vec<u8>,
}
impl RasterBiA
{
	pub fn new<P: AsRef<::std::fs::Path>>(path: P, fg: ::surface::Colour, bg: ::surface::Colour) -> Result<RasterBiA,LoadError> {
		use ::byteorder::{LittleEndian,ReadBytesExt};
		let path = path.as_ref();
		let mut file = try!( ::std::fs::File::open(path) );
		let w = try!( file.read_u16::<LittleEndian>() ) as usize;
		let h = try!( file.read_u16::<LittleEndian>() ) as usize;

		let mut data = Vec::with_capacity(w * h);
		let mut alpha = Vec::with_capacity(w * h);
		for _ in 0 .. w * h
		{
			let v = try!( file.read_u8() );
			data.push( v >= 128 );
			alpha.push( (v & 0x7F) * 2 | ((v >> 6) & 1) );
		}
		Ok(RasterBiA {
			bg: bg,
			fg: fg,
			width: w,
			data: data,
			alpha: alpha,
			})
	}
}
impl Buffer for RasterBiA {
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
				//kernel_log!("c = {:x}, alpha = {}", c.as_argb32(), a);
				*d = Colour::blend_alpha( Colour::from_argb32(*d), c, 255 - a ).as_argb32();
			}
			});
	}
}

