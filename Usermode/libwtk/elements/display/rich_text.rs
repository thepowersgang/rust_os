
pub struct RichText {
}
impl RichText {
	pub fn new() -> RichText {
		RichText {
		}
	}
}
impl crate::Element for RichText {
	fn render(&self, surface: crate::surface::SurfaceView, force: bool) {
		todo!()
	}

	fn resize(&self, _w: u32, _h: u32) {
		todo!()
	}

	fn with_element_at_pos(&self, pos: crate::geom::PxPos, dims: crate::geom::PxDims, f: crate::WithEleAtPosCb) -> bool {
		todo!()
	}
}