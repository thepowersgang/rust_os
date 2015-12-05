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
	selected_id: usize,

	header_dirty: bool,
	column_titles: Titles,
	widths_dirty: bool,
	column_widths: Vec<u32>,
	
	items: VecRing<(usize, Row)>,
}

impl<Titles: RowItems, Row: RowItems> ListView<Titles, Row>
{
	pub fn new(titles: Titles) -> Self {
		ListView {
			column_widths: RowItemIter::new(&titles).map(|x| (x.chars().count()*8+3) as u32).collect(),

			view_offset: 0,
			selected_id: 0,
			header_dirty: true,
			column_titles: titles,
			widths_dirty: true,
			items: VecRing::new(),
		}
	}
	/// Clear all state, ready for a fresh set of items
	pub fn clear(&mut self) {
		self.view_offset = 0;
		self.selected_id = 0;
		self.items = VecRing::new();
	}
	/// Clear cached names (e.g. item list has updated)
	pub fn refresh(&mut self) {
		self.items = VecRing::new();
	}
	/// Called to update the locally stored items
	pub fn maybe_resize<F>(&mut self, height: u32, mut get_row: F) -> bool
	where
		F: FnMut(usize) -> Option<(usize, Row)>
	{
		if height < self.row_height() {
			return false;
		}
		let new_count: usize = ((height - self.row_height()) / self.row_height()) as usize;
		if new_count != self.items.len() {
			self.items = VecRing::with_capacity(new_count);
			for idx in 0 .. new_count {
				if let Some(v) = get_row(idx) {
					self.items.push_back( v );
				}
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
	fn colour_sel_bg(&self) -> Colour {
		Colour::from_argb32(0x0000E0)
	}
	fn colour_sel_fg(&self) -> Colour {
		Colour::from_argb32(0xFFFFFF)
	}

	pub fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool)
	{
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
		let mut max_y = self.row_height();
		for (&(idx, ref row), y) in ::iterx::zip( &self.items, (1 .. ).map(|x| x*self.row_height()) )
		{
			kernel_log!("ListView::render - idx={}, row=['{}',...]", idx, row.value(0));
			if force || self.widths_dirty || true
			{
				let (c_bg, c_fg) = if idx == self.selected_id {
						(self.colour_sel_bg(), self.colour_sel_fg())
					}
					else {
						(self.colour_body_bg(), self.colour_body_fg())
					};
				//let row_view = surface.slice_ofs( Rect::new(0, y, !0, self.row_height()), 0, self.view_offset );
				let row_view = surface.slice( Rect::new(0, y, !0, self.row_height()) );
				row_view.fill_rect(Rect::new(0,0,!0,!0), c_bg);
				let mut x = 0;
				for (value, &width) in ::iterx::zip( RowItemIter::new(row), &self.column_widths )
				{
					if x > 0 {
						row_view.fill_rect(Rect::new(x+1, 0, 1, !0), c_fg);
					}
					row_view.draw_text(Rect::new(x+3, 0, width-3, !0), value.chars(), c_fg);
					x += width;
				}
			}

			max_y = y + self.row_height();
			if max_y >= surface.height() {
				break;
			}
		}

		// 3. Clear bottom area
		if force
		{
			if max_y < surface.height()
			{
				let view = surface.slice(Rect::new(0, max_y, !0,!0));
				view.fill_rect(Rect::new_full(), self.colour_body_bg() );
				let mut x = 0;
				for &width in &self.column_widths
				{
					if x > 0 {
						view.fill_rect(Rect::new(x+1, 0, 1, !0), self.colour_body_fg());
					}
					x += width;
				}
			}
		}
	}
}



impl RowItems for String {
	fn count(&self) -> usize { 1 }
	fn value(&self, _idx: usize) -> &str { &self }
}
impl<'a> RowItems for &'a str {
	fn count(&self) -> usize { 1 }
	fn value(&self, _idx: usize) -> &str { *self }
}
impl<T: AsRef<str>> RowItems for Vec<T> {
	fn count(&self) -> usize { self.len() }
	fn value(&self, idx: usize) -> &str { self[idx].as_ref() }
}
macro_rules! impl_rowitems_arrays {
	($($n:expr),*) => { $(
		impl<T: AsRef<str>> RowItems for [T; $n] {
			fn count(&self) -> usize { $n }
			fn value(&self, idx: usize) -> &str { self[idx].as_ref() }
		})*
	}
}

impl_rowitems_arrays! { 1, 2, 3, 4, 5, 6, 7, 8 }
macro_rules! impl_rowitems_tuple {
	( @forallsub $s:ident : $n1:ident = $v1:expr ) => {
		impl_rowitems_tuple! { $s : $n1 = $v1 }
	};
	( @forallsub $s:ident : $n1:ident = $v1:expr, $($n:ident = $v:expr),* ) => {
		impl_rowitems_tuple! { $s : $n1 = $v1, $($n = $v),* }
		impl_rowitems_tuple! { @forallsub $s : $($n = $v),* }
	};
	( $s:ident : $($n:ident = $v:expr),* ) => {
		impl<$($n: AsRef<str>),*> RowItems for ($($n,)*) {
			fn count(&$s) -> usize { 0 $( + {let _ = $v; 1})*  }
			fn value(&$s, idx: usize) -> &str { let mut v = $s.count(); $(v -= 1; if idx == v { return $v.as_ref(); } )* panic!("") }
		}
	};
}

impl_rowitems_tuple!{ @forallsub self : E = self.4, D = self.3, C = self.2, B = self.1, A = self.0 }

