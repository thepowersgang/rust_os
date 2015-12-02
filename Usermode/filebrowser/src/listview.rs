//
//
//
///
use wtk::Colour;
use wtk::geom::Rect;
use vec_ring::VecRing;

pub trait RowItems
{
	fn count(&self) -> usize;
	fn value(&self, idx: usize) -> &str;
}
//impl<const N: usize> RowItems for [&'static str; N] {
//}
impl RowItems for Vec<String> {
	fn count(&self) -> usize {
		self.len()
	}
	fn value(&self, idx: usize) -> &str {
		&self[idx]
	}
}
struct RowItemIter<'a, T: 'a + RowItems> {
	p: &'a T,
	i: usize,
}
impl<'a, T: 'a + RowItems> RowItemIter<'a, T> {
	fn new(p: &T) -> RowItemIter<T> {
		RowItemIter { p: p, i: 0 }
	}
}

impl<'a, T: 'a + RowItems> Iterator for RowItemIter<'a, T> {
	type Item = &'a str;
	fn next(&mut self) -> Option<&'a str> {
		if self.i == self.p.count() {
			None
		}
		else {
			self.i += 1;
			Some(self.p.value(self.i-1))
		}
	}
}


pub struct ListView<Titles: RowItems, Row: RowItems>
{
	view_offset: usize,

	header_dirty: bool,
	column_titles: Titles,
	widths_dirty: bool,
	column_widths: Vec<u32>,
	
	items: VecRing<Row>,
}

impl<Titles: RowItems, Row: RowItems> ListView<Titles, Row>
{
	pub fn new(titles: Titles) -> Self {
		ListView {
			view_offset: 0,
			header_dirty: true,
			column_titles: titles,
			widths_dirty: true,
			column_widths: Vec::new(),
			items: VecRing::new(),
		}
	}
	/// Called to update the locally stored items
	pub fn maybe_resize<F>(&mut self, height: u32, mut get_row: F) -> bool
	where
		F: FnMut(usize) -> Row
	{
		if height < self.row_height() {
			return false;
		}
		let new_count: usize = ((height - self.row_height()) / self.row_height()) as usize;
		if new_count != self.items.len() {
			self.items = VecRing::with_capacity(new_count);
			for idx in 0 .. new_count {
				self.items.push_back( get_row(idx) );
			}
			true
		}
		else {
			false
		}
	}

	//pub fn scroll_down<F>(&mut self, pixels: u32, mut get_row: F)
	//where
	//	F: FnMut(usize) -> Row
	//{
	//}

	fn row_height(&self) -> u32 {
		16+1
	}


	fn colour_header_bg(&self) -> Colour {
		Colour::from_argb32(0xD0D0D0)
	}
	fn colour_header_fg(&self) -> Colour {
		Colour::from_argb32(0x000000)
	}
	fn colour_body_bg(&self) -> Colour {
		Colour::from_argb32(0xFFFFFF)
	}
	fn colour_body_fg(&self) -> Colour {
		Colour::from_argb32(0x000000)
	}
}
impl<Titles: RowItems, Row: RowItems> ::wtk::Element for ListView<Titles, Row>
{
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
		// 1. Render the header
		if force || self.header_dirty || self.widths_dirty
		{
			let row_rect = Rect::new(0, 0, !0, self.row_height());
			surface.fill_rect(row_rect, self.colour_header_bg());
			let mut x = 0;
			let y = 0;	// constant
			for (title, &width) in ::iterx::zip( RowItemIter::new(&self.column_titles), &self.column_widths )
			{
				if x > 0 {
					surface.fill_rect(Rect::new(x+1, y, 1, self.row_height()), self.colour_header_fg());
				}
				surface.draw_text(Rect::new(x+3, y, width-3, self.row_height()), title.chars(), self.colour_header_fg());
				x += width;
			}
		}
		// 2. Render each item
		for (row, y) in ::iterx::zip( &self.items, (1 .. ).map(|x| x*self.row_height()) )
		{
			if force || true
			{
				//let row_view = surface.slice_ofs( Rect::new(0, y, !0, self.row_height()), 0, self.view_offset );
				let row_view = surface.slice( Rect::new(0, y, !0, self.row_height()) );
				row_view.fill_rect(Rect::new(0,0,!0,!0), Colour::theme_text_bg());
				let mut x = 0;
				for (value, &width) in ::iterx::zip( RowItemIter::new(row), &self.column_widths )
				{
					if x > 0 {
						row_view.fill_rect(Rect::new(x+1, 0, 1, !0), self.colour_body_fg());
					}
					row_view.draw_text(Rect::new(x+3, 0, width-3, !0), value.chars(), self.colour_body_fg());
					x += width;
				}
			}
		}
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::wtk::Element, (u32,u32)) {
		(self, (0,0))
	}
}

