
#[derive(Clone)]
pub struct UnwindState {
	regs: [u32; 16],
	vsp: u32,
}

#[derive(Debug)]
pub enum Error
{
	Refuse,	// Not an error
	Malformed,
	BadPointer(*const (),usize),
	Todo,
}

macro_rules! getreg {
	($r:ident) => {{ let v; asm!( concat!("mov $0, ", stringify!($r)) : "=r"(v)); v }};
}

//const TRACE_OPS: bool = true;
const TRACE_OPS: bool = false;

fn prel31(addr: usize, v: u32) -> usize {
	if v > 0x4000_0000 {
		usize::wrapping_add(addr, (v | 0x8000_0000) as usize)
	}
	else {
		usize::wrapping_add(addr, v as usize)
	}
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
	pub fn from_regs(regs: [u32; 16]) -> UnwindState {
		UnwindState {
			regs: regs,
			vsp: regs[13],
		}
	}

	pub fn get_ip(&self) -> u32 { self.regs[15] }
	pub fn get_lr(&self) -> u32 { self.regs[14] }
	
	pub fn unwind_step(&mut self, info: &u32) -> Result<(),Error> {
		let base = info as *const _ as usize;
		let info = *info;
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
			let ofs = (info | (1 << 31)) as usize;
			let ptr = usize::wrapping_add(base, ofs) as *const u32;
			//log_debug!("ptr = {:#x} + {:#x} = {:p}", base, ofs, ptr);
			// SAFE: Validity checked
			let word = unsafe {
				if ! ::memory::virt::is_reserved(ptr) {
					log_error!("BUG: Malformed entry at {:#x} - ptr={:p}", base+4, ptr);
					return Err( Error::Malformed );
				}
				*ptr
				};
			if word & 0x8000_0000 != 0 {
				if (word >> 28) & 0xF != 0x8 {
					log_error!("BUG: Malformed entry at {:p}: SBZ bits set 0x{:x} != 0x8", ptr, word >> 28);
					return Err( Error::Malformed );
				}
				let personality = (word >> 24) & 0xF;
				let words = if personality == 1 || personality == 2 {
						let word_count = (word >> 16) & 0xff;
						// SAFE: Will be checked
						let words_ptr = unsafe { ptr.offset(1) };
						// SAFE: Lifetime is 'static, data is POD
						match unsafe { ::memory::buf_to_slice(words_ptr, word_count as usize) }
						{
						Some(b) => b,
						None => {
							log_error!("BUG: Malformed entry at {:p}: {} words not valid afterwards", ptr, word_count);
							return Err( Error::Malformed );
							},
						}
					}
					else {
						&[] as &[u32]
					};

				match personality
				{
				0 => {
					try!( self.unwind_short16(word) );
					},
				1 => {
					try!( self.unwind_long16(word, words) );
					},
				2 => {
					// SAFE: Lifetime is 'static, data is POD
					try!( self.unwind_long32(word, words) );
					},
				v @ _ => {
					log_error!("TODO: Handle extra-word compact v={}", v);
					return Err( Error::Todo );
					},
				}
			}
			else {
				let addr = prel31(ptr as usize, word);
				log_error!("TODO: Custom exception routine? word={:#x} - addr={:#x}", word, addr);
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
				log_error!("BUG: Stack pointer {:p} invalid", ptr);
				return Err( Error::BadPointer(ptr as *const (), 4) );
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
			let count = (byte & 0x3F) as u32 * 4 + 4;
			if TRACE_OPS {
				log_debug!("VSP += {:#x}*4+4 ({})", byte & 0x3F, count);
			}
			self.vsp += count;
			},
		0x4 ... 0x7 => {	// ARM_EXIDX_CMD_DATA_PUSH
			let count = (byte & 0x3F) as u32 * 4 + 4;
			if TRACE_OPS {
				log_debug!("VSP -= {:#x}*4+4 ({})", byte & 0x3F, count);
			}
			self.vsp -= count;
			},
		0x8 => {	// ARM_EXIDX_CMD_REG_POP
			let extra = try!( getb() );
			if byte == 0x80 && extra == 0x00 {
				// Refuse to unwind
				return Err( Error::Refuse );
			}
			if TRACE_OPS {
				log_debug!("POP mask {:#x}{:02x}", byte & 0xF, extra);
			}
	
			// Lowest register at lowest stack
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
			if TRACE_OPS {
				log_debug!("VSP = R{}", byte & 0xF);
			}
			self.vsp = self.regs[(byte & 0xF) as usize];
			},
		0xA => {	// ARM_EXIDX_CMD_REG_POP
			let pop_lr = byte & 0x8 != 0;
			let count = (byte&0x7) as usize;
			if TRACE_OPS {
				log_debug!("POP {{r4-r{}{}}}", 4 + count, if pop_lr { ",lr" } else { "" });
			}
			for r in 4 .. 4 + count + 1 {
				self.regs[r] = try!(self.pop());
			}
			if pop_lr { self.regs[14] = try!(self.pop()); }
			},
		0xB => match byte & 0xF
			{
			0 => return Ok(true),	// ARM_EXIDX_CMD_FINISH
			1 => {
				let extra = try!( getb() );
				if extra == 0 {
					log_error!("EXIDX opcode 0xB1 {:#02x} listed as Spare", extra);
				}
				else if extra & 0xF0 != 0 {
					log_error!("EXIDX opcode 0xB1 {:#02x} listed as Spare", extra);
				}
				else {
					// Pop registers
					if TRACE_OPS {
						log_debug!("POP mask 0-3 {:#x}", extra & 0xFF);
					}
					if extra & 0x1 != 0 { self.regs[0] = try!(self.pop()); }	// R0
					if extra & 0x2 != 0 { self.regs[1] = try!(self.pop()); }	// R1
					if extra & 0x4 != 0 { self.regs[2] = try!(self.pop()); }	// R2
					if extra & 0x8 != 0 { self.regs[3] = try!(self.pop()); }	// R3
				}
				},
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

	fn getb<I: Iterator<Item=u8>>(it: &mut I) -> Result<u8,Error> {
		match it.next()
		{
		Some(v) => {
			//log_trace!("(G) byte {:#x}", v);
			Ok(v)
			},
		None => {
			log_warning!("Out of bytes for unwind mid-instruction");
			Err( Error::Malformed )
			},
		}
	}

	pub fn unwind_short16(&mut self, instrs: u32) -> Result<(), Error> {
		let mut it = WordBytesLE(instrs, 3);
		while let Some(b) = it.next()
		{
			if try!( self.unwind_instr(b, || Self::getb(&mut it)) ) {
				break ;
			}
		}
		Ok( () )
	}
	pub fn unwind_long16(&mut self, instrs: u32, extra: &[u32]) -> Result<(), Error> {
		let mut it = WordBytesLE(instrs, 2).chain( extra.iter().flat_map(|w| WordBytesLE(*w, 4)) );
		while let Some(b) = it.next()
		{
			if try!( self.unwind_instr(b, || Self::getb(&mut it)) ) {
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
		//log_trace!("self = ({:#x},{})", self.0, self.1);
		if self.1 == 0 {
			None
		}
		else {
			self.1 -= 1;
			Some( (self.0 >> (8 * self.1 as usize)) as u8 )
		}
	}
}


pub fn get_unwind_info_for(addr: usize) -> Option<(usize, &'static u32)>
{
	extern "C" {
		static __exidx_start: [u32; 2];
		static __exidx_end: ::Void;
	}

	let base = &__exidx_start as *const _ as usize;
	// SAFE: 'static slice
	let exidx_tab: &[ [u32; 2] ] = unsafe { ::core::slice::from_raw_parts(&__exidx_start, (&__exidx_end as *const _ as usize - base) / (2*4)) };

	let mut best = (0,0);
	// Locate the closest entry before the return address
	for (i,e) in exidx_tab.iter().enumerate() {
		assert!(e[0] < 0x8000_0000);
		
		let fcn_start = usize::wrapping_add( (e[0] as usize + 0x8000_0000), (&e[0] as *const _ as usize) );
		// If before the address, but after the previous closest
		if fcn_start < addr && fcn_start > best.0 {
			// then use it
			//log_trace!("{}: Use fcn_start={:#x}", i, fcn_start);
			best = (fcn_start, i);
		}
		else {
			//log_trace!("{}: Skip fcn_start={:#x}", i, fcn_start);
		}
	}
	//log_debug!("get_unwind_info_for({:#x}) : best = ({:#x}, {})", addr, best.0, best.1);
	if best.0 == 0 {
		None
	}
	else {
		Some( (best.0, &exidx_tab[best.1][1]) )
	}
}
