
pub fn rgba_to_u32(p: image::Rgba<u8>) -> u32 {
	(p.0[0] as u32) << 0
		| (p.0[1] as u32) << 8
		| (p.0[2] as u32) << 16
		| (p.0[3] as u32) << 24
}
