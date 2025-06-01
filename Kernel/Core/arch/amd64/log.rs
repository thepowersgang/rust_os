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
			fg_color: u32,
			decode_state: DecodeState,
		}
		enum DecodeState {
			Idle,
			EscapeSeen,	// "\x1B"
			Extended([u32; 4], u8),	// "\x1B["	- Holds the arguments and current index
		}
		const COLOR_WHITE: u32 = 0xFF_FF_FF;
		static STATE: crate::sync::Spinlock<State> = crate::sync::Spinlock::new(State {
			cursor_row: 0,
			cursor_col: 0,
			kf: crate::metadevs::video::kernel_font::KernelFont::new(0),
			fg_color: COLOR_WHITE,
			decode_state: DecodeState::Idle,
		});

		let Some(mut state) = STATE.try_lock_cpu() else { return ; };
		let state = &mut *state;
		const FONT_W: usize = 8;
		const FONT_H: usize = 16;
		let n_cols_cell = width / FONT_W;
		let n_rows_px = buf.len() / width;
		let n_rows_cell = n_rows_px / FONT_H;

		// Relatively crude ANSI escape code parsing
		// - Reduces noise
		match state.decode_state {
		DecodeState::Idle => match c {
			b'\x1b' => { state.decode_state = DecodeState::EscapeSeen; },
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
			},
		DecodeState::EscapeSeen => match c
			{
			b'[' => state.decode_state = DecodeState::Extended([0; 4], 0),
			_ => {
				// Ignore.
				state.decode_state = DecodeState::Idle;
			}
			},
		DecodeState::Extended(ref mut d, ref mut i) => {
			if let Some(v) = (c as char).to_digit(10) {
				if let Some(s) = d.get_mut(*i as usize) {
					// Don't want to panic, so use a really conservative limit
					if *s < 0xFFFF {
						*s *= 10;
						*s += v;
					}
				}
			}
			else if c == b';' {
				if (*i as usize) < d.len() {
					*i += 1;
				}
			}
			else {
				match c {
				b'm' => {
					for &v in d[..*i as usize].iter() {
						match v {
						// Reset
						0 => {
							//state.fg_alt = false;
							state.fg_color = COLOR_WHITE;
						},
						//1 => { state.fg_alt = true; },
						// Colours
						30 => { state.fg_color = 0xFF_FF_FF; }	// White
						31 => { state.fg_color = 0xFF_00_00; }	// Red
						32 => { state.fg_color = 0x00_FF_00; }	// Green
						33 => { state.fg_color = 0xFF_FF_00; }	// Yellow
						34 => { state.fg_color = 0x00_00_FF; }	// Blue
						35 => { state.fg_color = 0xFF_00_FF; }	// Purple
						_ => {},
						}
					}
				},
				_ => {
					// Ignore.
				}
				}
				state.decode_state = DecodeState::Idle;
			}
		}
		}
		
	})
}

// vim: ft=rust

