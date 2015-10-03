
pub struct UnwindState {
	regs: [u32; 16],
	vsp: u32,
}

pub enum Error
{
	Refuse,	// Not an error
	Malformed,
	Todo,
}

macro_rules! getreg {
	($r:ident) => {{ let v; asm!( concat!("mov $0, ", stringify!($r)) : "=r"(v)); v }};
}

impl UnwindState {
	#[inline(always)]
	pub fn new_cur() -> UnwindState {
		// SAFE: Just reads register states
		unsafe {
			UnwindState {
				regs: [
					getreg!(r0), getreg!(r1), getreg!(r2), getreg!(r3),
					getreg!(r4), getreg!(r5), getreg!(r6), getreg!(r7),
					getreg!(r8), getreg!(r9), getreg!(r10), getreg!(r11),
					getreg!(r12), getreg!(sp), getreg!(lr), getreg!(pc),
					],
				vsp: { let v; asm!("mov $0, sp" : "=r" (v)); v },
			}
		}
	}

	pub fn get_ip(&self) -> u32 { self.regs[15] }
	pub fn get_lr(&self) -> u32 { self.regs[14] }
	
	pub fn unwind_step(&mut self, info: &[u32; 2]) -> Result<(),Error> {
		let base = &info[0] as *const _ as usize + 4;
		let info = info[1];
		if info == 0x1 {
			// Can't unwind
			return Err( Error::Refuse );
		}
		else if (info >> 31) == 1 {
			// Inline information
			if info >> 24 != 0x80 {
				log_error!("BUG: Malformed entry at {:#x}: SBZ bits set 0x{:x} != 0x8", base+4, info >> 24);
				return Err( Error::Malformed );
			}
			try!( self.unwind_short16(info) );
		}
		else {
			// Indirect pointer
			let ptr = (base + info as usize + 0x8000_0000) as *const u32;
			// SAFE: Validity checked
			let word = unsafe {
				if ! ::memory::virt::is_reserved(ptr) {
					return Err( Error::Malformed );
				}
				*ptr
				};
			if word & 0x8000_0000 != 0 {
				if (word >> 28) & 0xF != 0x8 {
					log_error!("BUG: Malformed entry at {:p}: SBZ bits set 0x{:x} != 0x8", ptr, word >> 28);
					return Err( Error::Malformed );
				}
				match (word >> 24) & 0xF
				{
				0 => {
					try!( self.unwind_short16(word) );
					},
				1 => {
					let word_count = (word >> 16) & 0xff;
					// SAFE: Lifetime is 'static, data is POD
					try!( self.unwind_long16(word, unsafe { try!(::memory::buf_to_slice(ptr.offset(1), word_count as usize).ok_or(Error::Malformed)) }) );
					},
				2 => {
					let word_count = (word >> 16) & 0xff;
					// SAFE: Lifetime is 'static, data is POD
					try!( self.unwind_long32(word, unsafe { try!(::memory::buf_to_slice(ptr.offset(1), word_count as usize).ok_or(Error::Malformed)) }) );
					},
				v @ _ => {
					log_error!("TODO: Handle extra-word compact v={}", v);
					return Err( Error::Todo );
					},
				}
			}
			else {
				log_error!("TODO: Custom exception routine? word={:#x}", word);
				return Err( Error::Todo );
			}
		}

		Ok( () ) 
	}

	fn pop(&mut self) -> Result<u32,Error> {
		// SAFE: Memory is present
		let v = unsafe {
			let ptr = self.vsp as *const u32;
			if ! ::memory::virt::is_reserved(ptr) {
				return Err( Error::Malformed );
			}
			*ptr
			};
		self.vsp += 4;
		Ok( v )
	}


	
	/// Returns `true` if instruction stream is complete
	fn unwind_instr<F>(&mut self, byte: u8, mut getb: F) -> Result<bool,Error>
	where
		F: FnMut() -> Result<u8,Error>
	{
		match byte >> 4
		{
		0x0 ... 0x3 => {	// ARM_EXIDX_CMD_DATA_POP
			log_debug!("POP data {:#x}*4+4", byte & 0x3F);
			self.vsp += (byte & 0x3F) as u32 * 4 + 4;
			},
		0x4 ... 0x7 => {	// ARM_EXIDX_CMD_DATA_PUSH
			log_debug!("PUSH data {:#x}*4+4", byte & 0x3F);
			self.vsp -= (byte & 0x3F) as u32 * 4 + 4;
			},
		0x8 => {	// ARM_EXIDX_CMD_REG_POP
			let extra = try!( getb() );
			if byte == 0x80 && extra == 0x00 {
				// Refuse to unwind
				return Err( Error::Refuse );
			}
			log_debug!("POP mask {:#x}{:02x}", byte & 0xF, extra);

			if extra & 0x01 != 0 { self.regs[4] = try!(self.pop()); }	// R4
			if extra & 0x02 != 0 { self.regs[5] = try!(self.pop()); }	// R5
			if extra & 0x04 != 0 { self.regs[6] = try!(self.pop()); }	// R6
			if extra & 0x08 != 0 { self.regs[7] = try!(self.pop()); }	// R7
			if extra & 0x10 != 0 { self.regs[8] = try!(self.pop()); }	// R8
			if extra & 0x20 != 0 { self.regs[9] = try!(self.pop()); }	// R9
			if extra & 0x40 != 0 { self.regs[10] = try!(self.pop()); }	// R10
			if extra & 0x80 != 0 { self.regs[11] = try!(self.pop()); }	// R11
			if byte & 0x1 != 0 { self.regs[12] = try!(self.pop()); }	// R12
			if byte & 0x2 != 0 { self.regs[13] = try!(self.pop()); }	// R13
			if byte & 0x4 != 0 { self.regs[14] = try!(self.pop()); }	// R14
			if byte & 0x8 != 0 { self.regs[15] = try!(self.pop()); }	// R15
			},
		0x9 => {	// ARM_EXIDX_CMD_REG_TO_SP
			log_debug!("VSP = R{}", byte & 0xF);
			self.vsp = self.regs[(byte & 0xF) as usize];
			},
		0xA => {	// ARM_EXIDX_CMD_REG_POP
			let pop_lr = byte & 0x8 != 0;
			let count = (byte&0x7) as usize;
			log_debug!("POP {{r4-r{}{}}}", 4 + count, if pop_lr { ",lr" } else { "" });
			for r in 4 .. 4 + count {
				self.regs[r] = try!(self.pop());
			}
			if pop_lr { self.regs[14] = try!(self.pop()); }
			},
		0xB => match byte & 0xF
			{
			0 => return Ok(true),	// ARM_EXIDX_CMD_FINISH
			_ => {
				log_error!("TODO: EXIDX opcode {:#02x}", byte);
				return Err( Error::Todo );
				},
			},
		_ => {
			log_error!("TODO: EXIDX opcode {:#02x}", byte);
			return Err( Error::Todo );
			},
		}
		Ok( false )
	}
	pub fn unwind_short16(&mut self, instrs: u32) -> Result<(), Error> {
		let mut it = WordBytesLE(instrs, 3);
		while let Some(b) = it.next()
		{
			if try!( self.unwind_instr(b, || it.next().ok_or( Error::Malformed )) ) {
				break ;
			}
		}
		Ok( () )
	}
	pub fn unwind_long16(&mut self, instrs: u32, extra: &[u32]) -> Result<(), Error> {
		let bytes = [ ((instrs >> 8) & 0xFF) as u8, (instrs & 0xFF) as u8 ];
		let mut it = WordBytesLE(instrs, 2).chain( extra.iter().flat_map(|w| WordBytesLE(*w, 4)) );
		while let Some(b) = it.next()
		{
			if try!( self.unwind_instr(b, || it.next().ok_or( Error::Malformed )) ) {
				break ;
			}
		}
		Ok( () )
	}
	pub fn unwind_long32(&mut self, instrs: u32, extra: &[u32]) -> Result<(), Error> {
		log_error!("TODO: unwind_long32");
		Err( Error::Todo )
	}
}

struct WordBytesLE(u32, u8);
impl ::core::iter::Iterator for WordBytesLE {
	type Item = u8;
	fn next(&mut self) -> Option<u8> {
		if self.1 == 0 {
			None
		}
		else {
			self.1 -= 1;
			Some( (self.0 >> (8 * self.1 as usize)) as u8 )
		}
	}
}

