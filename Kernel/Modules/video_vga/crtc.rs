// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/vga/crtc.rs
// - VGA CRTC Handler
//
use kernel::prelude::*;
use kernel::lib::UintBits;
use kernel::arch::x86_io;

pub struct CrtcRegs
{
	regs: [u8; 0x20],
}

pub const PIX_PER_CHAR: u16 = 16;

impl CrtcRegs
{
	pub fn load(base: u16) -> CrtcRegs
	{
		let mut rv = CrtcRegs {
			regs: [0; 0x20],
			};
		rv.read(base);
		rv
	}
	
	// CR0: H Total
	pub fn set_h_total(&mut self, val: u16)
	{
		assert!(val <= 0xFF);
		self.regs[0] = (val & 0xFF) as u8;
	}
	// CR1: H Display End
	pub fn set_h_disp_end(&mut self, val: u16)
	{
		assert!(val <= 0xFF);
		self.regs[1] = (val & 0xFF) as u8;
	}
	// CR2: H Blank Start
	pub fn set_h_blank_start(&mut self, val: u16)
	{
		assert!(val <= 0xFF);
		self.regs[2] = (val & 0xFF) as u8;
	}
	// CR3: H Blank Length
	pub fn set_h_blank_len(&mut self, val: u16)
	{
		assert!(val <= 0x3F);
		self.regs[3] &= !(0x1F << 0);
		self.regs[3] |= (val & 0x1F) as u8;
		self.regs[0x5] &= !(1 << 7);
		self.regs[0x5] |= (val.bit(5) as u8) << 7;
	}
	// CR4: H Sync Start
	pub fn set_h_sync_start(&mut self, val: u16)
	{
		assert!(val <= 0x1FF);
		self.regs[4] = (val & 0xFF) as u8;
		self.regs[0x1A] &= !(1 << 4);
		self.regs[0x1A] |= (val.bit(8) as u8) << 4;
	}
	// CR5: H Sync End
	pub fn set_h_sync_end(&mut self, val: u16)
	{
		assert!(val <= 0x1F);
		self.regs[5] &= !(0x1F);
		self.regs[5] &= (val & 0x1F) as u8;
	}
	// CR6: V Total
	pub fn set_v_total(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[6] = (val & 0xFF) as u8;
		self.regs[7] &= !( (1 << 0) | (1 << 5) );
		self.regs[7] |= (val.bit(8) << 0 | val.bit(9) << 5) as u8;
	}
	// CR12: V Display End
	pub fn set_v_disp_end(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x12] = (val & 0xFF) as u8;
		// CR7[1,6] := val[8,9]
		self.regs[0x07] &= !( 1 << 1 | 1 << 6 );
		self.regs[0x07] |= (val.bit(8) << 1 | val.bit(9) << 6) as u8;
	}
	// CR15: V Blank Start
	pub fn set_v_blank_start(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x15] = (val & 0xFF) as u8;
		self.regs[0x07] &= !( 1 << 3 );
		self.regs[0x07] |= (val.bit(8) as u8) << 3;
		self.regs[0x09] &= !( 1 << 5 );
		self.regs[0x09] |= (val.bit(9) as u8) << 5;
	}
	// CR16: V Blank End
	pub fn set_v_blank_end(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x16] = (val & 0xFF) as u8;
		self.regs[0x1A] &= !( 3 << 6 );
		self.regs[0x1A] |= (val.bits(8,9) as u8) << 6;
	}
	// CR10: V Sync Start
	pub fn set_v_sync_start(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x10] = (val & 0xFF) as u8;
		self.regs[0x07] &= !( 1 << 2 | 1 << 7 );
		self.regs[0x07] |= (val.bit(8) << 2 | val.bit(9) << 7) as u8;
	}
	// CR11: V Sync End
	pub fn set_v_sync_end(&mut self, val: u16)
	{
		assert!(val <= 0xF);
		self.regs[0x11] &= !( 0xF );
		self.regs[0x11] |= (val & 0xF) as u8;
	}
	// CR18: Line Compare - The scanline where ScreenA finishes (And ScreenB starts)
	pub fn set_line_compare(&mut self, val: u16)
	{
		assert!(val < 0x3FF);
		self.regs[0x18] = (val & 0xFF) as u8;
		self.regs[0x07] &= !( 1 << 4 );
		self.regs[0x07] |= (val.bit(8) as u8) << 4;
		self.regs[0x09] &= !( 1 << 6 );
		self.regs[0x09] |= (val.bit(9) as u8) << 6;
	}
	// CR13: Offset (vertical scrolling)
	pub fn set_offset(&mut self, val: u16)
	{
		assert!(val < 0x1FF);
		self.regs[0x13] = (val & 0xFF) as u8;
		self.regs[0x1B] &= !( 1 << 4 );
		self.regs[0x1B] |= (val.bit(8) as u8) << 4;
	}
	
	// CR8: Byte Pan
	pub fn set_byte_pan(&mut self, val: u8)
	{
		self.regs[8] &= !( 3 << 5 );
		self.regs[8] |= (val & 3) << 5
	}

	// CRC/CRD: Screen Start
	pub fn set_screen_start(&mut self, val: u16)
	{
		self.regs[0xC] = (val >> 8) as u8;
		self.regs[0xD] = (val & 0xFF) as u8;
	}
	
	
	fn read(&mut self, base: u16)
	{
		for (idx,val) in self.regs.iter_mut().enumerate()
		{
			// SAFE: Have a &mut, no race
			unsafe {
				x86_io::outb(base + 0, idx as u8);
				*val = x86_io::inb(base + 1);
			}
		}
	}

	pub fn commit(&mut self, base: u16)
	{
		for (idx,val) in self.regs.iter().enumerate()
		{
			// SAFE: Have a &mut, no race
			unsafe {
				x86_io::outb(base + 0, idx as u8);
				x86_io::outb(base + 1, *val);
			}
		}
	}
}

// vim: ft=rust
