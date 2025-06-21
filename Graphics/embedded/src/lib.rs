#![no_std]

pub mod font_cp437_8x16;
pub mod panic {
	include!{ concat!(env!("OUT_DIR"),"/panic.rs") }
}
pub mod logo {
	include!{ concat!(env!("OUT_DIR"),"/logo.rs") }
}

pub struct RleRow(&'static [u8], &'static [u32]);
impl RleRow {
	pub fn decompress(&self, dst: &mut [u32]) {
		assert!(self.0.len() == self.1.len());
		let mut j = 0;
		for i in 0 .. self.0.len() {
			let (&c,&v) = unsafe { (self.0.get_unchecked(i), self.1.get_unchecked(i)) };
			for _ in 0 .. c {
				dst[j] = v;
				j += 1;
			}
		}
	}
}