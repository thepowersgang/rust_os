// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/log.rs
//! RS232 logging output
#[allow(unused_imports)]
use crate::prelude::*;

use super::x86_io;

#[inline(never)]
/// Print a string to the logging output
pub fn puts(text: &str)
{
	for c in text.bytes()
	{
		putc(c);
	}
}
/// Print a hexadecimal value to the logging output
pub fn puth(val: u64) {
	let nibbles = {
		let mut v = 1;
		while v < 64/4 && (val >> v*4) > 0 { v += 1 }
		v
		};
	puts("0x");
	puth_digits(val, nibbles);
}
pub fn puth64(val: u64) {
	puth_digits(val, 64/4)
}
pub fn puth_digits(val: u64, nibbles: usize) {
	for i in 0 .. nibbles
	{
		let nibble : u8 = ((val >> (nibbles-i-1)*4) & 15) as u8;
		putc( if nibble <= 9 { '0' as u8 + nibble } else { 'a' as u8 + nibble-10 } );
	}
}
/// Print a single character to the logging output
#[inline(never)]
fn putc(c: u8)
{
	// SAFE: Racy I guess... but doesn't cause memory unsafety (and worst is lost charafter)
	unsafe {
		// Wait for the serial port to report ready to send
		while (x86_io::inb(0x3F8+5) & 0x20) == 0
		{
		}
		// Send to the serial port AND the bochs port 0xE9 hack
		x86_io::outb(0x3F8, c as u8);
		x86_io::outb(0xe9, c as u8);
	}

	// Early-boot logging
	// - For use before the video drivers are up
	super::boot::with_preboot_video(|buf, width| {
		struct State {
			cursor_row: usize,
			cursor_col: usize,
			kf: crate::metadevs::video::kernel_font::KernelFont,
		}
		static STATE: crate::sync::Spinlock<State> = crate::sync::Spinlock::new(State {
			cursor_row: 0,
			cursor_col: 0,
			kf: crate::metadevs::video::kernel_font::KernelFont::new(0),
		});
		let Some(mut state) = STATE.try_lock_cpu() else { return ; };
		const FONT_W: usize = 8;
		const FONT_H: usize = 16;
		let n_cols_cell = width / FONT_W;
		let n_rows_px = buf.len() / width;
		let n_rows_cell = n_rows_px / FONT_H;

		// TODO: Could add decoding for ANSI escape sequences, but that's more complex than is needed for this code.
		match c {
		b'\n' => {
			state.cursor_row += 1;
			state.cursor_col = 0;
			buf[ (state.cursor_row % n_rows_cell * FONT_H) * width ..][..FONT_H * width].fill(0);
		},
		_ => {
			if state.cursor_col >= n_cols_cell {
				state.cursor_row += 1;
				state.cursor_col = 0;
				buf[ (state.cursor_row % n_rows_cell * FONT_H) * width ..][..FONT_H * width].fill(0);
			}
			let col = state.cursor_col;
			state.cursor_col += 1;
			let row = state.cursor_row % n_rows_cell;
			let buf = &mut buf[row * FONT_H * width + col * FONT_W..];
			state.kf.putc(0xFF_FF_FF, c as char, |_| {});
			state.kf.putc(0xFF_FF_FF, ' ', |d| {
				for (i,r) in d.chunks(8).enumerate() {
					buf[i * width..][..FONT_W].copy_from_slice(r);
				}
			});
		}
		}
	})
}

// vim: ft=rust

