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
		Colour::from_argb32(0xFF_B0B0B0)
	}
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

const MIN_HANDLE_LENGTH: u32 = 7;	// 3 lines, padding*2, border*2
const ARROW_SIZE: u32 = 5;	// 5px high/wide arrow

trait Direction: Default
{
	/// Turn cartesian into short/long coords
	fn to_sl(w: u32, h: u32) -> (u32,u32);
	/// Turn short/long into cartesian
	fn from_sl(short: u32, long: u32) -> (u32,u32);
	fn draw_arrow(suraface: ::surface::SurfaceView, side_is_low: bool);
	fn draw_grip(suraface: ::surface::SurfaceView);
}

#[derive(Default)]
pub struct Vertical;
impl Direction for Vertical
{
	fn to_sl(w: u32, h: u32) -> (u32,u32) {
		(w, h)
	}
	fn from_sl(short: u32, long: u32) -> (u32,u32) {
		(short, long)
	}
	fn draw_arrow(surface: ::surface::SurfaceView, side_is_low: bool)
	{
		let midpt_x = surface.width() / 2;
		let midpt_y = surface.height() / 2;
		let surface = surface.slice(Rect::new(midpt_x - ARROW_SIZE, midpt_y - ARROW_SIZE, ARROW_SIZE*2,ARROW_SIZE*2));

		for row in 0 .. ARROW_SIZE {
			let npx = if side_is_low { (row + 1) * 2 } else { ARROW_SIZE*2 - row * 2 };
			surface.fill_rect( Rect::new(ARROW_SIZE - npx/2, row, npx, 1), FixedTheme.control_detail() );
		}
	}
	fn draw_grip(surface: ::surface::SurfaceView)
	{
		let surface = surface.slice(Rect::new(0, surface.height()/2 - MIN_HANDLE_LENGTH/2, !0,MIN_HANDLE_LENGTH));

		// Three lines at +1, +3, +5
		surface.fill_rect( Rect::new(2,1, surface.width()-4,1), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(2,3, surface.width()-4,1), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(2,5, surface.width()-4,1), FixedTheme.control_detail() );
	}
}
#[derive(Default)]
pub struct Horizontal;
impl Direction for Horizontal
{
	fn to_sl(w: u32, h: u32) -> (u32,u32) {
		(h, w)
	}
	fn from_sl(short: u32, long: u32) -> (u32,u32) {
		(long, short)
	}
	fn draw_arrow(surface: ::surface::SurfaceView, side_is_low: bool)
	{
		let midpt_x = surface.width() / 2;
		let midpt_y = surface.height() / 2;
		let surface = surface.slice(Rect::new(midpt_x - ARROW_SIZE, midpt_y - ARROW_SIZE, ARROW_SIZE*2,ARROW_SIZE*2));

		for row in 0 .. ARROW_SIZE {
			let npx = row;
			let startx = if side_is_low { ARROW_SIZE - row } else { 0 };
			surface.fill_rect( Rect::new(startx, row, npx, 1), FixedTheme.control_detail() );
			surface.fill_rect( Rect::new(startx, ARROW_SIZE*2 - row, npx, 1), FixedTheme.control_detail() );
		}
		surface.fill_rect( Rect::new(0, ARROW_SIZE, ARROW_SIZE, 1), FixedTheme.control_detail() );
	}
	fn draw_grip(surface: ::surface::SurfaceView)
	{
		let surface = surface.slice(Rect::new(surface.width()/2 - MIN_HANDLE_LENGTH/2,0, MIN_HANDLE_LENGTH,!0));

		// Three lines at +1, +3, +5
		surface.fill_rect( Rect::new(1,2, 1, surface.height()-4), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(3,2, 1, surface.height()-4), FixedTheme.control_detail() );
		surface.fill_rect( Rect::new(5,2, 1, surface.height()-4), FixedTheme.control_detail() );
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
		let (sd, ld) = D::to_sl(w, h);
		let mut state = self.state.borrow_mut();
		state.dirty = true;
		state.bar_cap = ld - 2*sd;
		state.recalculate();
	}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		if force {
			surface.fill_rect(Rect::new(0,0,!0,!0), FixedTheme.control_background());
		}
		let (short_d, long_d) = D::to_sl(surface.width(), surface.height());
		if force {
			D::draw_arrow(surface.slice(Rect::new(0,0, short_d,short_d)), true);
		}
		let mut state = self.state.borrow_mut();
		if force || ::std::mem::replace(&mut state.dirty, false) {
			// Background
			let (x,y) = D::from_sl(0, short_d);
			let (w,h) = D::from_sl(short_d, long_d - 2*short_d);
			surface.fill_rect(Rect::new(x,y, w,h), FixedTheme.control_background());

			// Indicator outline
			let (x,y) = D::from_sl(0, short_d + state.bar_start);
			let (w,h) = D::from_sl(short_d, state.bar_size);
			let r = Rect::new(x,y, w,h);
			surface.draw_rect(r, ::geom::Px(1), FixedTheme.control_detail());
			// Indicator grip
			D::draw_grip( surface.slice(r) );
		}
		if force {
			let (x,y) = D::from_sl(0, long_d - short_d);
			D::draw_arrow(surface.slice(Rect::new(x,y, short_d,short_d)), false);
		}
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::Element,(u32,u32)) {
		(self, (0,0))
	}
}


impl State
{
	fn recalculate(&mut self)
	{
		fn div_round(n: u64, d: u64) -> u64 {
			(n + d/2) / d
		}

		if self.value_limit == 0 {
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
	}
}

