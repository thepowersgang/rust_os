

// Bitmap font used by this module is in another file
include!("../../../../Graphics/font_cp437_8x16.rs");

pub struct KernelFont([u32; 8*16], u32);

impl KernelFont
{
	pub fn new(background: u32) -> KernelFont {
		KernelFont([background; 8*16], background)
	}
	pub fn get_buf(&self) -> &[u32; 8*16] {
		&self.0
	}

	pub fn putc<F>(&mut self, colour: u32, c: char, mut blit: F)
	where
		F: FnMut(&[u32; 8*16])
	{
		// If the character was a combining AND it's not at the start of a line,
		// render atop the previous cell
		if c.is_combining() {
			self.render_char(colour, c);
		}
		// Otherwise, wipe the cell and render into it
		else {
			blit(&self.0);
			self.0 = [self.1; 8*16];
			self.render_char(colour, c);
		}
	}

	/// Actually does the rendering
	fn render_char(&mut self, colour: u32, cp: char)
	{
		let idx = unicode_to_cp437(cp);
		let bitmap = &S_FONTDATA[idx as usize];
		
		// Actual render!
		for row in 0 .. 16
		{
			let byte = &bitmap[row];
			let r = &mut self.0[row * 8 ..][.. 8];
			for col in 0 .. 8
			{
				if (byte >> 7-col) & 1 != 0 {
					r[col] = colour;
				}
			}
		}
	}
}

/// Trait to provde 'is_combining', used by render code
trait UnicodeCombining
{
	fn is_combining(&self) -> bool;
}
impl UnicodeCombining for char
{
	fn is_combining(&self) -> bool
	{
		match *self as u32
		{
		// Ranges from wikipedia:Combining_Character
		0x0300 ..= 0x036F => true,
		0x1AB0 ..= 0x1AFF => true,
		0x1DC0 ..= 0x1DFF => true,
		0x20D0 ..= 0x20FF => true,
		0xFE20 ..= 0xFE2F => true,
		_ => false,
		}
	}
}
