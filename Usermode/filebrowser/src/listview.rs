//
//
//
///
use wtk::Colour;
use wtk::geom::Rect;
use std::cell::Cell;

pub trait Row
{
	//fn is_dirty(&self) -> bool;
	fn count(&self) -> usize;
	fn value(&self, idx: usize) -> &str;
}

struct RowItemIter<'a, T: 'a + Row> {
	p: &'a T,
	i: usize,
}
impl<'a, T: 'a + Row> RowItemIter<'a, T> {
	fn new(p: &T) -> RowItemIter<T> {
		RowItemIter { p: p, i: 0 }
	}
}

impl<'a, T: 'a + Row> Iterator for RowItemIter<'a, T> {
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


pub struct ListView<Titles: Row, R: Row>
{
	view_offset: Cell<usize>,
	selected_id: Cell<usize>,

	header_dirty: Cell<bool>,
	widths_dirty: Cell<bool>,
	items_replaced: Cell<bool>,

	column_titles: Titles,
	column_widths: Vec<u32>,
	
	items: ::std::cell::RefCell<Vec<R>>,
}

impl<Titles: Row, R: Row> ListView<Titles, R>
{
	pub fn new(titles: Titles) -> Self {
		ListView {
			column_widths: RowItemIter::new(&titles).map(|x| (x.chars().count()*8+3) as u32).collect(),

			view_offset: Default::default(),
			selected_id: Default::default(),
			header_dirty: Cell::new(true),
			widths_dirty: Cell::new(true),
			items_replaced: Cell::new(true),
			column_titles: titles,
			items: Default::default(),
		}
	}
	/// Clear all state, ready for a fresh set of items
	pub fn clear(&self) {
		self.items_replaced.set(true);
		self.view_offset.set( 0 );
		self.selected_id.set( 0 );
		*self.items.borrow_mut() = Vec::new();
	}
	pub fn append_item(&self, item: R) {
		self.items.borrow_mut().push(item);
	}

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
}

impl<Titles: Row, R: Row> ListView<Titles, R>
{
	// NOTE: VERY VERY EVIL - The passed closure returns Option<OtherCLosure> where OtherClosure can mutate the item list
	pub fn handle_event<F,C>(&self, event: ::wtk::InputEvent, mut open_cb: F) -> bool
	where
		F: FnMut(&R)->Option<C>,
		C: FnOnce()
	{
		match event
		{
		//::wtk::InputEvent::MouseClick(x,y,b) =>
		//::wtk::InputEvent::MouseUp(x,y,b) => {
		//
		//	},

		::wtk::InputEvent::KeyUp(key) =>
			match key
			{
			::wtk::KeyCode::UpArrow =>
				if self.selected_id.get() > 0 {
					self.selected_id.set( self.selected_id.get() - 1 );
					true
				}
				else {
					false
				},
			::wtk::KeyCode::DownArrow =>
				if self.selected_id.get() < self.items.borrow().len() {
					self.selected_id.set( self.selected_id.get() + 1 );
					true
				}
				else {
					false
				},
			::wtk::KeyCode::Return => {
				let c = open_cb( &self.items.borrow()[self.selected_id.get()]);
				if let Some(c) = c {
					c();
					true
				}
				else {
					false
				}
				},
			_ => false,
			},
		_ => false,
		}
	}
	pub fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool)
	{
		// 1. Render the header
		if force || self.header_dirty.get() || self.widths_dirty.get()
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
		for (idx, (row, y)) in (0 .. ).zip( ::iterx::zip( self.items.borrow().iter(), (1 .. ).map(|x| x*self.row_height()) ) )
		{
			if force || self.widths_dirty.get() || true //row.is_dirty()
			{
				let (c_bg, c_fg) = if idx == self.selected_id.get() {
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
		if force || self.widths_dirty.get() || self.items_replaced.get()
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

		self.items_replaced.set(false);
		self.widths_dirty.set(false);
		self.header_dirty.set(false);
	}
}



impl Row for String {
	fn count(&self) -> usize { 1 }
	fn value(&self, _idx: usize) -> &str { &self }
}
impl<'a> Row for &'a str {
	fn count(&self) -> usize { 1 }
	fn value(&self, _idx: usize) -> &str { *self }
}
impl<T: AsRef<str>> Row for Vec<T> {
	fn count(&self) -> usize { self.len() }
	fn value(&self, idx: usize) -> &str { self[idx].as_ref() }
}
macro_rules! impl_rowitems_arrays {
	($($n:expr),*) => { $(
		impl<T: AsRef<str>> Row for [T; $n] {
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
		impl<$($n: AsRef<str>),*> Row for ($($n,)*) {
			fn count(&$s) -> usize { 0 $( + {let _ = $v; 1})*  }
			fn value(&$s, idx: usize) -> &str { let mut v = $s.count(); $(v -= 1; if idx == v { return $v.as_ref(); } )* panic!("") }
		}
	};
}

impl_rowitems_tuple!{ @forallsub self : E = self.4, D = self.3, C = self.2, B = self.1, A = self.0 }

