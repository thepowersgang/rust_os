//
//
//

pub struct Image;

impl Image
{
	//pub fn new<P: AsRef<::std::path::Path>>(file: P) -> Image {
	pub fn new(_file: &str) -> Image {
		Image
	}
}

impl ::Element for Image
{
	fn focus_change(&self, _have: bool) {
	}

	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool {
		false
	}

	fn render(&self, surface: ::surface::SurfaceView) {
	}
}


