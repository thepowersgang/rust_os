//
//
//
//! Generic directional scrollbar
use geom::Rect;
use surface::Colour;

trait Theme
{
	fn control_detail(&self) -> Colour;
	fn control_background(&self) -> Colour;
}
struct FixedTheme;
impl Theme for FixedTheme {
	fn control_detail(&self) -> Colour {
		Colour::from_argb32(0xFF_909090)
	}
	//fn control_detail_disabled(&self) -> Colour {
	//	Colour::from_argb32(0xFF_C0C0C0)
	//}
	fn control_background(&self) -> Colour {
		Colour::from_argb32(0xFF_F0F0F0)
	}
}

pub struct Widget<D>
{
	dir: D,
	state: ::std::cell::RefCell<State>,
}
#[derive(Default)]
struct State
{
	dirty: bool,

	/// Current scrollbar mid position (in value-space)
	value_cur: usize,
	/// Size of the scrollbar
	value_size: usize,
	/// Maximum value for `value_cur` (sum of value_cur and value_size shouldn't exceed this)
	value_limit: usize,


	/// Total number of pixels available in the bar region
	bar_cap: u32,

	// (calculated) 
	bar_start: u32,
	bar_size: u32,
}

const MIN_HANDLE_LENGTH: u32 = 11;	// 3 lines, padding*4, border*2
const ARROW_SIZE: u32 = 5;	// 5px high/wide arrow

pub trait Direction: Default
{
	/// Turn cartesian into short/long coords
	fn to_sl(&self, w: u32, h: u32) -> (u32,u32);
	/// Turn short/long into cartesian
	fn from_sl(&self, short: u32, long: u32) -> (u32,u32);
	fn draw_arrow(&self, suraface: ::surface::SurfaceView, side_is_low: bool);
	fn draw_grip(&self, suraface: ::surface::SurfaceView);
	fn name(&self) -> &'static str;
}

#[derive(Default)]
pub struct Vertical;
impl Direction for Vertical
{
	fn to_sl(&self, w: u32, h: u32) -> (u32,u32) {
		(w, h)
	}
	fn from_sl(&self, short: u32, long: u32) -> (u32,u32) {
		(short, long)
	}
	fn draw_arrow(&self, surface: ::surface::SurfaceView, side_is_low: bool)
	{
		let midpt_x = surface.width() / 2;
		let midpt_y = surface.height() / 2;
		let surface = surface.slice(Rect::new(midpt_x - ARROW_SIZE, midpt_y - ARROW_SIZE/2, ARROW_SIZE*2,ARROW_SIZE*2));

		for row in 0 .. ARROW_SIZE {
			let npx = if side_is_low { (row + 1) * 2 } else { ARROW_SIZE*2 - row * 2 };
			surface.fill_rect( Rect::new(ARROW_SIZE - npx/2, row, npx, 1), FixedTheme.control_detail() );
		}
	}
	fn draw_grip(&self, surface: ::surface::SurfaceView)
	{
		const SIZE: u32 = 7;
		const MARGIN: u32 = 2;
		let surface = surface.slice(Rect::new(0, surface.height()/2 - SIZE/2, !0,SIZE));

		// Three lines at +1, +3, +5
		surface.fill_rect( Rect::new(MARGIN,1, surface.width()-MARGIN*2,1), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(MARGIN,3, surface.width()-MARGIN*2,1), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(MARGIN,5, surface.width()-MARGIN*2,1), FixedTheme.control_detail() );
	}
	fn name(&self) -> &'static str {
		"vert"
	}
}
#[derive(Default)]
pub struct Horizontal;
impl Direction for Horizontal
{
	fn to_sl(&self, w: u32, h: u32) -> (u32,u32) {
		(h, w)
	}
	fn from_sl(&self, short: u32, long: u32) -> (u32,u32) {
		(long, short)
	}
	fn draw_arrow(&self, surface: ::surface::SurfaceView, side_is_low: bool)
	{
		let midpt_x = surface.width() / 2;
		let midpt_y = surface.height() / 2;
		let surface = surface.slice(Rect::new(midpt_x - ARROW_SIZE/2, midpt_y - ARROW_SIZE, ARROW_SIZE*2,ARROW_SIZE*2));

		for row in 0 .. ARROW_SIZE {
			let npx = row;
			let startx = if side_is_low { ARROW_SIZE - row } else { 0 };
			surface.fill_rect( Rect::new(startx, row, npx, 1), FixedTheme.control_detail() );
			surface.fill_rect( Rect::new(startx, ARROW_SIZE*2 - row, npx, 1), FixedTheme.control_detail() );
		}
		surface.fill_rect( Rect::new(0, ARROW_SIZE, ARROW_SIZE, 1), FixedTheme.control_detail() );
	}
	fn draw_grip(&self, surface: ::surface::SurfaceView)
	{
		const SIZE: u32 = 7;
		const MARGIN: u32 = 2;
		let surface = surface.slice(Rect::new(surface.width()/2 - SIZE/2,0, SIZE,!0));

		// Three lines at +2, +4, +6
		surface.fill_rect( Rect::new(1,MARGIN, 1, surface.height()-MARGIN*2), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(3,MARGIN, 1, surface.height()-MARGIN*2), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(5,MARGIN, 1, surface.height()-MARGIN*2), FixedTheme.control_detail() );
	}
	fn name(&self) -> &'static str {
		"horiz"
	}
}

impl<D> Widget<D>
where
	D: Direction
{
	fn new_() -> Widget<D>
	{
		Widget {
			state: Default::default(),
			dir: D::default(),
			}
	}

	/// Set (capacity, handke_size)
	/// NOTE: The size must be <= the capacity
	pub fn set_bar(&self, bar: Option<(usize, usize)>) {
		let mut s = self.state.borrow_mut();
		if let Some( (cap, size) ) = bar {
			assert!( size <= cap );
			s.value_limit = cap;
			s.value_size = size;
		}
		else {
			s.value_limit = 0;
			s.value_size = 1;
		}
		s.recalculate();
	}
	pub fn set_pos(&self, cap: usize) {
		let mut st = self.state.borrow_mut();
		st.value_cur = cap;
		st.recalculate();
	}
	pub fn get_pos(&self) -> usize {
		self.state.borrow().value_cur
	}
}
impl Widget<Vertical> {
	pub fn new() -> Widget<Vertical> {
		Self::new_()
	}
}
impl Widget<Horizontal> {
	pub fn new() -> Widget<Horizontal> {
		Self::new_()
	}
}

impl<D> ::Element for Widget<D>
where
	D: Direction
{
	fn resize(&self, w: u32, h: u32) {
		let (sd, ld) = self.dir.to_sl(w, h);
		let mut state = self.state.borrow_mut();
		state.dirty = true;
		state.bar_cap = ld - 2*sd;
		state.recalculate();
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		if force {
			surface.fill_rect(Rect::new(0,0,!0,!0), FixedTheme.control_background());
		}
		let (short_d, long_d) = self.dir.to_sl(surface.width(), surface.height());
		if force {
			let r = Rect::new(0,0, short_d,short_d);
			kernel_log!("{} start r={:?}", self.dir.name(), r);
			surface.draw_rect(r, ::geom::Px(1), FixedTheme.control_detail());
			self.dir.draw_arrow(surface.slice(r), true);
		}
		let mut state = self.state.borrow_mut();
		if force || ::std::mem::replace(&mut state.dirty, false) {
			// Background
			let (x,y) = self.dir.from_sl(0, short_d);
			let (w,h) = self.dir.from_sl(short_d, long_d - 2*short_d);
			surface.fill_rect(Rect::new(x,y, w,h), FixedTheme.control_background());

			// Indicator outline
			let (x,y) = self.dir.from_sl(0, short_d + state.bar_start);
			let (w,h) = self.dir.from_sl(short_d, state.bar_size);
			let r = Rect::new(x,y, w,h);
			kernel_log!("{} bar r={:?}", self.dir.name(), r);
			surface.draw_rect(r, ::geom::Px(1), FixedTheme.control_detail());
			// Indicator grip
			self.dir.draw_grip( surface.slice(r) );
		}
		if force {
			let (x,y) = self.dir.from_sl(0, long_d - short_d);
			let r = Rect::new(x,y, short_d,short_d);
			kernel_log!("{} end r={:?}", self.dir.name(), r);
			surface.draw_rect(r, ::geom::Px(1), FixedTheme.control_detail());
			self.dir.draw_arrow(surface.slice(r), false);
		}
	}
	fn with_element_at_pos(&self, pos: ::geom::PxPos, _dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool {
		f(self, pos)
	}
}


impl State
{
	fn recalculate(&mut self)
	{
		fn div_round(n: u64, d: u64) -> u64 {
			(n + d/2) / d
		}

		if self.value_limit == 0 || self.value_size > self.value_limit {
			self.bar_start = 0;
			self.bar_size = self.bar_cap;
		}
		else {
			let start = div_round(self.value_cur  as u64 * self.bar_cap as u64, self.value_limit as u64) as u32;
			let size = div_round(self.value_size as u64 * self.bar_cap as u64, self.value_limit as u64) as u32;

			if size < MIN_HANDLE_LENGTH {
				let midpt = start + size / 2;
				if midpt < MIN_HANDLE_LENGTH/2 {
					self.bar_start = 0;
				}
				else {
					self.bar_start = midpt - MIN_HANDLE_LENGTH/2;
				}
				self.bar_size = MIN_HANDLE_LENGTH;
			}
			else {
				self.bar_start = start;
				self.bar_size = size;
			}
		}

		kernel_log!("State = {{ bar_start: {}, bar_size: {}, bar_cap: {} }}",
			self.bar_start, self.bar_size, self.bar_cap);
	}
}

