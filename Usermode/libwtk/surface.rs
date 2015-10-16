
use geom::{Rect,Px};

#[derive(Copy,Clone)]
pub struct Colour(u32);
impl Colour
{
	pub fn from_argb32(argb32: u32) -> Colour { Colour(argb32) }
	pub fn from_rgb(r: u8, g: u8, b: u8) -> Colour {
		let argb32 = 0 << 24 | (r as u32) << 16 | (g as u32) << 8 | (b as u32);
		Colour( argb32 )
	}
	//pub fn black() -> Colour { Colour(0) }
	//pub fn ltgray() -> Colour { Colour(0xDD_DD_DD) }
	//pub fn gray() -> Colour { Colour(0x55_55_55) }
	//pub fn white() -> Colour { Colour(0xFF_FF_FF) }
	pub fn as_argb32(&self) -> u32 { self.0 }

	pub fn theme_text() -> Colour { Colour(0x00_000000) }
	pub fn theme_text_alt() -> Colour { Colour(0x00_606060) }
	pub fn theme_border_main() -> Colour { Colour(0x00_C0C0C0) }
	pub fn theme_border_alt() -> Colour { Colour(0x00_E0E0E0) }
	pub fn theme_text_bg() -> Colour { Colour(0xF8FFF8) }
	pub fn theme_body_bg() -> Colour { Colour(0x002000) }

	/// Alpha value, 0 = opaque, 255 = transparent
	pub fn alpha(&self) -> u8 {
		(self.0 >> 24) as u8
	}
	pub fn red  (&self) -> u8 { (self.0 >> 16) as u8 }
	pub fn green(&self) -> u8 { (self.0 >>  8) as u8 }
	pub fn blue (&self) -> u8 { (self.0 >>  0) as u8 }

	pub fn blend_alpha(lower: Colour, upper: Colour, alpha: u8) -> Colour {
		let alpha: u32 = alpha as u32;
		if alpha == 0 {
			upper
		}
		else if alpha == 255 {
			lower
		}
		else {
			let r = Self::blend_component( alpha, lower.red(),   upper.red() );
			let g = Self::blend_component( alpha, lower.green(), upper.green() );
			let b = Self::blend_component( alpha, lower.blue(),  upper.blue() );
			Colour::from_rgb(r,g,b)
		}
	}
	pub fn blend(lower: Colour, upper: Colour) -> Colour {
		Colour::blend_alpha(lower, upper, upper.alpha())
	}
	fn blend_component(alpha: u32, lower: u8, upper: u8) -> u8 {
		let val_by_255 = lower as u32 * alpha + upper as u32 * (255 - alpha);
		(val_by_255 / 255) as u8
	}
}

#[derive(Default)]
pub struct Surface
{
	width: usize,
	dirty: ::std::cell::Cell<Rect<Px>>,
	data: ::std::cell::RefCell<Vec<u32>>,
}

impl Surface
{
	fn height(&self) -> u32 {
		if self.width == 0 {
			assert_eq!(self.data.borrow().len(), 0);
			0
		}
		else {
			(self.data.borrow().len() / self.width) as u32
		}
	}

	/// Blit to the passed GUI window
	pub fn blit_to_win(&self, win: &::syscalls::gui::Window)
	{
		let dirty: Rect<Px> = self.dirty.get();
		self.dirty.set(Default::default());
		let first_row = dirty.y().0 as usize;
		let row_count = dirty.height().0 as usize;
		let first_col = dirty.x().0 as usize;
		let col_count = dirty.width().0 as usize;
		
		// Blit just the dirty region
		win.blit_rect(
			first_col as u32, first_row as u32,
			col_count as u32, row_count as u32,
			&self.data.borrow()[first_row*self.width + first_col .. ][ .. row_count*self.width],
			self.width
			);
	}
	pub fn invalidate_all(&mut self) {
		self.dirty.set( self.rect() );
	}
	/// Resize the surface (clearing existing content)
	pub fn resize(&mut self, dims: ::syscalls::gui::Dims, fill: Colour) {
		self.width = dims.w as usize;
		*self.data.borrow_mut() = vec![fill.as_argb32(); (dims.w as usize * dims.h as usize)];
		// On resize, set dirty area to full area of the surface
		self.invalidate_all();
	}
	/// Obtain a rect covering the entire surface
	pub fn rect(&self) -> Rect<Px> {
		Rect::new(0, 0, self.width as u32, self.height())
	}
	/// Obtain a view into this surface
	pub fn slice(&self, rect: Rect<Px>) -> SurfaceView {
		let rect = self.rect().intersect(&rect);
		//kernel_log!("Surface::slice - rect={:?}", rect);
		SurfaceView { surf: self, rect: rect }
	}

	fn foreach_scanlines<F: FnMut(usize, &mut [u32])>(&self, rect: Rect<Px>, mut f: F) {
		// Update dirty region with this rect
		let rect = self.rect().intersect(&rect);
		//let mut dr = self.dirty.borrow_mut();
		if self.dirty.get().is_empty() {
			self.dirty.set(rect);
		}
		else {
			self.dirty.set( self.dirty.get().union(&rect) ); 
		}

		//kernel_log!("foreach_scanlines(rect={:?}, F={})", rect, type_name!(F));
		for (i, row) in self.data.borrow_mut().chunks_mut(self.width).skip(rect.y().0 as usize).take(rect.height().0 as usize).enumerate()
		{
			//kernel_log!("{}: {}  {}..{} row.len()={}", i, rect.y().0 as usize + i, rect.x().0, rect.x2().0, row.len());
			f( i, &mut row[rect.x().0 as usize .. rect.x2().0 as usize] );
		}
		//kernel_log!("- done");
	}
}

pub struct SurfaceView<'a>
{
	surf: &'a Surface,
	rect: Rect<Px>,
}
impl<'a> SurfaceView<'a>
{
	/// Obtain a full rectangle of this surface
	pub fn rect(&self) -> Rect<Px> {
		Rect::new(0, 0, self.width(), self.height())
	}
	pub fn width(&self) -> u32 { self.rect.width().0 }
	pub fn height(&self) -> u32 { self.rect.height().0 }

	/// Create a sub-view of the surface
	pub fn slice(&self, rect: Rect<Px>) -> SurfaceView {
		SurfaceView {
			surf: self.surf,
			rect: self.rect.intersect(&rect.offset(self.rect.x(), self.rect.y())),
		}
	}

	/// Iterate over scanlines in a rect (scanlines are [u32] xRGB32)
	pub fn foreach_scanlines<F: FnMut(usize, &mut [u32])>(&self, rect: Rect<Px>, f: F) {
		self.surf.foreach_scanlines( self.rect.relative(&rect), f )
	}

	/// Fill a region with a solid colour
	pub fn fill_rect(&self, rect: Rect<Px>, colour: Colour) {
		self.foreach_scanlines(rect, |_, line|
			for px in line.iter_mut() {
				*px = colour.as_argb32();
			}
			);
	}
	pub fn draw_rect(&self, rect: Rect<Px>, lw: Px, colour: Colour) {
		let lwu = lw.0 as usize;
		assert!(lwu > 0);
		self.foreach_scanlines(rect, |i, line|
			if i < lwu || i >= rect.h.0 as usize - lwu {
				for px in line.iter_mut() {
					*px = colour.as_argb32();
				}
			}
			else {
				for px in line[.. lwu].iter_mut() {
					*px = colour.as_argb32();
				}
				for px in line[rect.w.0 as usize - lwu .. ].iter_mut() {
					*px = colour.as_argb32();
				}
			}
			);
	}

	/// Draw characters yielded from the passed iterator using the default font
	pub fn draw_text<It: Iterator<Item=char>>(&self, mut rect: Rect<Px>, chars: It, colour: Colour) -> usize {
		let mut st = S_FONT.get_renderer();
		let mut chars = chars.peekable();
		//kernel_log!("draw_text: rect = {:?}", rect);
		while let Some( (w,_h) ) = st.render_grapheme(&mut chars, colour)
		{
			self.foreach_scanlines(rect, |i, line| {
				//kernel_log!("i = {}, line.len() = {}", i, line.len());
				for (d,s) in line.iter_mut().zip( st.buffer(i, w as usize) )
				{
					*d = Colour::blend( Colour::from_argb32(*d), Colour::from_argb32(*s) ).as_argb32();
				}
				});
			rect = rect.offset(::geom::Px(w), ::geom::Px(0));
		}
		rect.x().0 as usize
	}
}

// --------------------------------------------------------------------
// Fallback/simple monospace font (Classic VGA, aka CP437)
// --------------------------------------------------------------------

static S_FONT: MonoFont = MonoFont::new();
struct MonoFont;
impl MonoFont {
	const fn new() -> MonoFont { MonoFont }
	fn get_renderer(&self) -> MonoFontRender {
		MonoFontRender { buffer: [0; 8*16], }
	}
}

include!("../../Graphics/font_cp437_8x16.rs");

struct MonoFontRender {
	buffer: [u32; 8*16],
}
impl MonoFontRender
{
	pub fn render_grapheme<It: Iterator<Item=char>>(&mut self, it: &mut ::std::iter::Peekable<It>, colour: Colour) -> Option<(u32,u32)> {
		self.buffer = [0xFF_000000; 8*16];
		if let Some(ch) = it.next()
		{
			self.render_char(colour, ch);
			while it.peek().map(|c| c.is_combining()).unwrap_or(false)
			{
				self.render_char(colour, it.next().unwrap());
			}
			Some( (8,16) )
		}
		else {
			None
		}
	}
	pub fn buffer(&self, row: usize, width: usize) -> &[u32] {
		if row*8 >= self.buffer.len() {
			&[]
		}
		else {
			&self.buffer[row * 8..][..width]
		}
	}

	/// Actually does the rendering
	fn render_char(&mut self, colour: Colour, cp: char)
	{
		let idx = unicode_to_cp437(cp);
		//kernel_log!("render_char - '{}' = {:#x}", cp, idx);
		
		let bitmap = &S_FONTDATA[idx as usize];
		
		// Actual render!
		for row in (0 .. 16)
		{
			let byte = &bitmap[row as usize];
			let base = row * 8;
			let r = &mut self.buffer[base .. base + 8]; 
			for col in (0usize .. 8)
			{
				if (byte >> 7-col) & 1 != 0 {
					r[col] = colour.as_argb32();
				}
			}
		}
	}
}

/// Trait to provde 'is_combining', used by render code
trait UnicodeCombining
{
	fn is_combining(&self) -> bool;
}

impl UnicodeCombining for char
{
	fn is_combining(&self) -> bool
	{
		match *self as u32
		{
		// Ranges from wikipedia:Combining_Character
		0x0300 ... 0x036F => true,
		0x1AB0 ... 0x1AFF => true,
		0x1DC0 ... 0x1DFF => true,
		0x20D0 ... 0x20FF => true,
		0xFE20 ... 0xFE2F => true,
		_ => false,
		}
	}
}
