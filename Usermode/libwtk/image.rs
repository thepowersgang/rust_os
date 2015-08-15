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
		// Don't care
	}

	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool {
		// Don't care
		false
	}

	fn render(&self, surface: ::surface::SurfaceView) {
		// TODO: Render image
		surface.fill_rect( surface.rect(), ::surface::Colour::theme_text_bg() );
	}
}


