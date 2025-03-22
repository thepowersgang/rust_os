//
//
//
use geom::{Rect,Px};
use surface::Colour;

pub enum Align {
	Left,
	Center,
	Right,
}

//enum Tile {
//	None,
//	Stretch,
//	Repeat,
//}

/// Static image wrapper
pub struct Image<T: Buffer>
{
	has_changed: ::std::cell::Cell<bool>,
	align_v: Align,
	align_h: Align,
	data: T,
}

impl Align
{
	fn get_ofs(&self, item: u32, avail: u32) -> u32 {
		if item >= avail {
			return 0;
		}
		match self
		{
		&Align::Left => 0,
		&Align::Center => avail / 2 - item / 2,
		&Align::Right => avail - item,
		}
	}
}

impl<T: Buffer> Image<T>
{
	pub fn new(i: T) -> Image<T> {
		Image {
			has_changed: ::std::cell::Cell::new(true),
			data: i,
			align_h: Align::Center,
			align_v: Align::Center,
		}
	}

	/// Set the vertical alignment of the image
	pub fn set_align_v(&mut self, align: Align) {
		self.align_v = align;
		self.force_redraw();
	}
	/// Set the horizontal alignment of the image
	pub fn set_align_h(&mut self, align: Align) {
		self.align_h = align;
		self.force_redraw();
	}

	pub fn force_redraw(&self) {
		self.has_changed.set(true); 
	}
	pub fn dims_px(&self) -> (u32,u32) {
		let Rect { w: Px(w), h: Px(h), .. } = self.data.dims_px();
		(w, h)
	}
}

impl<T: Buffer> ::Element for Image<T>
{
	fn focus_change(&self, _have: bool) {
		// Don't care
	}

	fn handle_event(&self, _ev: ::InputEvent, _win: &mut dyn crate::window::WindowTrait) -> bool {
		// Don't care
		false
	}

	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		if force || self.has_changed.get() {
			let (i_w, i_h) = self.dims_px();
			let x = self.align_h.get_ofs(i_w, surface.width());
			let y = self.align_h.get_ofs(i_h, surface.height());
			let subsurf = surface.slice( Rect::new(Px(x), Px(y), Px(!0), Px(!0)) );
			self.data.render(subsurf);
			self.has_changed.set(false);
		}
	}
	fn resize(&self, _w: u32, _h: u32) {}
	fn with_element_at_pos(&self, pos: ::geom::PxPos, _dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool {
		f(self, pos)
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

fn get_4_bytes<F: ::std::io::Read>(f: &mut F) -> Result<[u8; 4], ::std::io::Error> {
	let mut rv = [0; 4];
	if f.read(&mut rv)? != 4 {
		todo!("Handle unexpected EOF in get_4_bytes");
	}
	Ok( rv )
}
fn get_fixed_vec<F: ::std::io::Read>(f: &mut F, size: usize) -> Result<Vec<u8>, ::std::io::Error> {
	let mut data: Vec<u8> = (0 .. size).map(|_| 0u8).collect();
	if f.read(&mut data)? != size {
		todo!("Handle unexpected EOF in get_fixed_vec");
	}
	Ok( data )
}

/// Full-colour raster image
pub struct RasterRGB
{
	width: usize,
	data: Vec<u8>,
}
impl RasterRGB
{
	pub fn new_img<P: AsRef<::std::fs::Path>>(path: P) -> Result<Image<Self>,LoadError> {
		Self::new(path).map(|b| Image::new(b))
	}
	pub fn new<P: AsRef<::std::fs::Path>>(path: P) -> Result<RasterRGB,LoadError> {
		use ::byteorder::{LittleEndian,ReadBytesExt};
		use std::io::Read;
		let path = path.as_ref();
		let mut file = ::std::io::BufReader::new( ::std::fs::File::open(path)? );
		// - Check magic
		let magic = get_4_bytes(&mut file)?;
		if &magic ==  b"\x7FR24" {
			// - Read dimensions
			let w = file.read_u16::<LittleEndian>()? as usize;
			let h = file.read_u16::<LittleEndian>()? as usize;
			kernel_log!("w = {}, h = {}", w, h);
			// - Read data
			let data = get_fixed_vec(&mut file, w*h*3)?;

			Ok(RasterRGB {
				width: w,
				data: data,
				})
		}
		else if &magic ==  b"\x7FR\x18R" {
			// - Read dimensions
			let w = file.read_u16::<LittleEndian>()? as usize;
			let h = file.read_u16::<LittleEndian>()? as usize;
			kernel_log!("w = {}, h = {}", w, h);
			let size = w*h*3;
			let mut data: Vec<u8> = (0 .. size).map(|_| 0u8).collect();
			let mut pos = 0;
			
			while pos < size
			{
				let count_u8 = file.read_u8()?;
				let px_buf = {
					let mut buf = [0; 3];
					if file.read(&mut buf)? != 3 {
						panic!("TODO: Handle unexpected EOF when parsing RLE");
					}
					buf
					};
				for _ in 0 .. count_u8 {
					data[pos..][..3].copy_from_slice(&px_buf);
					pos += 3;
				}
			}

			Ok(RasterRGB {
				width: w,
				data: data,
				})
		}
		else {
			kernel_log!("RasterRGB::new - Image magic ({:?}) bad", magic);
			Err(LoadError::Malformed)
		}
	}
}
impl Buffer for RasterRGB {
	fn dims_px(&self) -> Rect<Px> {
		Rect::new(0,0,  self.width as u32, (self.data.len() / 3 / self.width) as u32)
	}
	fn render(&self, buf: ::surface::SurfaceView) {
		kernel_log!("buf.rect() = {:?}, self.dims_px() = {:?}", buf.rect(), self.dims_px());
		let mut buf_rows = self.data.chunks(self.width*3);
		buf.foreach_scanlines(self.dims_px(), |_row, line| {
			let val = buf_rows.next().unwrap();
			for (d, px) in Iterator::zip( line.iter_mut(), val.chunks(3) )
			{
				let v = (px[0] as u32) << 16 | (px[1] as u32) << 8 | (px[2] as u32) << 0;
				*d = v;
			}
			});
	}
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
	pub fn new_img<P: AsRef<::std::fs::Path>>(path: P, fg: ::surface::Colour) -> Result<Image<Self>,LoadError> {
		Self::new(path, fg).map(|b| Image::new(b))
	}
	pub fn new<P: AsRef<::std::fs::Path>>(path: P, fg: ::surface::Colour) -> Result<RasterMonoA,LoadError> {
		use ::byteorder::{LittleEndian,ReadBytesExt};
		let path = path.as_ref();
		let mut file = ::std::fs::File::open(path)?;
		// - Check magic
		if &get_4_bytes(&mut file)? != b"\x7FR8M" {
			return Err(LoadError::Malformed);
		}
		// - Read dimensions
		let w = file.read_u16::<LittleEndian>()? as usize;
		let h = file.read_u16::<LittleEndian>()? as usize;
		// - Read data (directly)
		let alpha = get_fixed_vec(&mut file, w*h)?;
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
	pub fn new_img<P: AsRef<::std::fs::Path>>(path: P, fg: ::surface::Colour, bg: ::surface::Colour) -> Result<Image<Self>,LoadError> {
		Self::new(path, fg, bg).map(|b| Image::new(b))
	}
	pub fn new<P: AsRef<::std::fs::Path>>(path: P, fg: ::surface::Colour, bg: ::surface::Colour) -> Result<RasterBiA,LoadError> {
		use ::byteorder::{LittleEndian,ReadBytesExt};
		let path = path.as_ref();
		let mut file = ::std::fs::File::open(path)?;
		// - Check magic
		if &get_4_bytes(&mut file)? != b"\x7FR8B" {
			return Err(LoadError::Malformed);
		}
		// - Read dimensions
		let w = file.read_u16::<LittleEndian>()? as usize;
		let h = file.read_u16::<LittleEndian>()? as usize;

		let mut data = Vec::with_capacity(w * h);
		let mut alpha = Vec::with_capacity(w * h);
		for _ in 0 .. w * h
		{
			let v = file.read_u8()?;
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

