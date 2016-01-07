
pub struct Widget
{
	first_line: usize,

	lines: Vec<Line>
}

struct Line
{
	file_offset: u64,
	file_size: usize,	// May be != data.len() if the line wasn't valid UTF-8
	// TODO: Use ByteString or other - and do box-chars for invalid codepoints?
	data: String,
}


impl Widget
{
	pub fn new() -> Widget {
		Widget {
			first_line: 0,
			lines: Vec::new(),
			}
	}
}
impl ::wtk::Element for Widget
{
	fn resize(&self, _w: u32, _h: u32) {
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::wtk::Element,(u32,u32)) {
		(self, (0,0))
	}
}

