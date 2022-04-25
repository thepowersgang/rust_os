//! ARM EABI unwind table parsing
// See: ARM Doc IHI0038

#[derive(Clone)]
pub struct UnwindState {
	regs: [u32; 16],
	vsp: u32,

	ucb: _Unwind_Control_Block,
}
extern "C" {
	type _Unwind_Context;
}

#[allow(dead_code,non_camel_case_types)]
mod enums {
	pub type _Unwind_Reason_Code = i32;
	pub const _URC_OK                      : _Unwind_Reason_Code = 0;
	pub const _URC_FOREIGN_EXCEPTION_CAUGHT: _Unwind_Reason_Code = 1;
	pub const _URC_HANDLER_FOUND           : _Unwind_Reason_Code = 6;
	pub const _URC_INSTALL_CONTEXT         : _Unwind_Reason_Code = 7;
	pub const _URC_CONTINUE_UNWIND         : _Unwind_Reason_Code = 8;
	pub const _URC_FAILURE                 : _Unwind_Reason_Code = 9;
	pub type _Unwind_State = i32;
	pub const _US_VIRTUAL_UNWIND_FRAME : _Unwind_State = 0;
	pub const _US_UNWIND_FRAME_STARTING: _Unwind_State = 1;
	pub const _US_UNWIND_FRAME_RESUME  : _Unwind_State = 2;
}
use self::enums::*;
#[repr(C)]
#[derive(Clone)]
struct _Unwind_Control_Block
{
	exception_class: [u8; 8],
	exception_cleanup: extern "C" fn(_Unwind_Reason_Code, *mut _Unwind_Control_Block),
	unwinder_cache: [u32; 5],
	barrier_cache: [u32; 6],
	cleanup_cache: [u32; 4],
	pr_cache: _Unwind_Control_Block__PrCache,
}
impl Default for _Unwind_Control_Block {
	fn default() -> _Unwind_Control_Block {
		_Unwind_Control_Block {
			exception_class: [0; 8],
			exception_cleanup: { extern "C" fn cleanup(_: _Unwind_Reason_Code, _: *mut _Unwind_Control_Block) {} cleanup },
			unwinder_cache: Default::default(),
			barrier_cache: Default::default(),
			cleanup_cache: Default::default(),
			pr_cache: _Unwind_Control_Block__PrCache {
				fnstart: 0,
				ehtp: 0 as *mut _,
				additional: 0,
				_reserved: 0,
				},
		}
	}
}
#[repr(C)]
#[derive(Clone)]
struct _Unwind_Control_Block__PrCache
{
	fnstart: u32,
	ehtp: *mut _Unwind_EHT_Header,
	additional: u32,
	_reserved: u32,
}
#[allow(non_camel_case_types)]
type _Unwind_EHT_Header = u32;

#[derive(Debug)]
pub enum Error
{
	Refuse,	// Not an error
	Malformed,
	BadPointer(*const (),usize),
	Todo,
}

macro_rules! getreg {
	($r:ident) => {{ let v; ::core::arch::asm!( concat!("mov {0}, ", stringify!($r)), out(reg) v); v }};
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
				vsp: { let v; ::core::arch::asm!("mov {}, sp", lateout(reg) v); v },
				ucb: Default::default(),
			}
		}
	}
	pub fn from_regs(regs: [u32; 16]) -> UnwindState {
		UnwindState {
			regs: regs,
			vsp: regs[13],
			ucb: Default::default(),
		}
	}

	#[allow(dead_code)]	// Unused in kernel, used by userland
	pub fn get_ip(&self) -> u32 { self.regs[15] }
	pub fn get_lr(&self) -> u32 { self.regs[14] }
	
	/// Update the unwind state using the tables
	pub fn unwind_step(&mut self, info: &u32) -> Result<(),Error> {
		let base = info as *const _ as usize;
		let info = *info;
		if info == 0x1 /*EXIDX_CANTUNWIND*/ {
			// Can't unwind
			return Err( Error::Refuse );
		}
		else if (info >> 31) == 1 {
			// Inline information
			if info >> 24 != 0x80 {
				log_error!("BUG: Malformed entry at {:#x}: SBZ bits set 0x{:x} != 0x8", base+4, info >> 24);
				return Err( Error::Malformed );
			}
			self.unwind_short16(info)?;
		}
		else {
			// Indirect pointer (31-bit relative address)
			let ptr = prel31(base, info) as *const u32;
			// SAFE: Validity checked
			let word = unsafe {
				if ptr as usize & 3 != 0 || ! crate::memory::virt::is_reserved(ptr) {
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
						match unsafe { crate::memory::buf_to_slice(words_ptr, word_count as usize) }
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
					self.unwind_short16(word)?;
					},
				1 => {
					self.unwind_long16(word, words)?;
					},
				2 => {
					self.unwind_long32(word, words)?;
					},
				v @ _ => {
					log_error!("TODO: Handle extra-word compact v={}", v);
					return Err( Error::Todo );
					},
				}
			}
			// Top bit unset: A custom personality routine 
			else {
				let addr = prel31(ptr as usize, word);
				//// Call the handling routine
				//// SAFE: Trusting the tables
				//let cb: extern"C" fn( _Unwind_State, *mut _Unwind_Control_Block, *mut _Unwind_Context )->_Unwind_Reason_Code = unsafe { ::core::mem::transmute(addr) };
				//match cb(_US_VIRTUAL_UNWIND_FRAME, &mut self.ucb as *mut _, &mut self.regs as *mut _ as *mut _)
				//{
				//_URC_CONTINUE_UNWIND => {
				//	},
				//_URC_HANDLER_FOUND => { log_error!("Found a `catch`?"); return Err(Error::Todo); },
				//_URC_FAILURE => { log_error!("PR failure"); return Err(Error::Malformed); }
				//_ => return Err(Error::Malformed),
				//}
				extern "C" {
					fn rust_eh_personality();
				}
				// __gnu_unwind frame is called by libstd's rust_eh_personality impl
				// - This in turn is just a simple wrapper around the ARM format
				if addr == rust_eh_personality as usize {
					// Run the GNU unwinder
					// See gcc/libgcc/config/arm/pr-support.c
					struct GnuUnwind<'a> {
						data: u32,
						bytes_left: u8,
						next: &'a [u32],
					}
					impl<'a> GnuUnwind<'a> {
						fn getb(&mut self) -> Option<u8> {
							if self.bytes_left == 0 {
								let (d, n) = self.next.split_first()?;
								self.data = *d;
								self.next = n;
								self.bytes_left = 3;
							}
							else {
								self.bytes_left -= 1;
							}
							let rv = (self.data >> 24) as u8;
							self.data <<= 8;
							//log_debug!("getb=0x{:02x}", rv);
							Some(rv)
						}
					}
					// SAFE: Trusting the compiler here
					let mut gnu = unsafe {
						let ptr = ptr.offset(1);	// Skip the personality function pointer
						GnuUnwind {
							data: *ptr << 8,
							bytes_left: 3,
							next: ::core::slice::from_raw_parts(ptr.offset(1), (*ptr >> 24) as usize),
							}
						};
					
					while self.unwind_instr(gnu.getb().unwrap_or(0xB0), || gnu.getb().ok_or(Error::Malformed))? == false
					{
					}
				}
				else {
					log_error!("TODO: Properly call the handler routine (seems to directly call ::unwind::rust_eh_personality? - {:#x}", addr);
					return Err(Error::Todo);
				}
			}
		}

		Ok( () ) 
	}

	fn pop(&mut self) -> Result<u32,Error> {
		// SAFE: Memory is present
		let v = unsafe {
			let ptr = self.vsp as *const u32;
			if ! crate::memory::virt::is_reserved(ptr) {
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
		0x0 ..= 0x3 => {	// ARM_EXIDX_CMD_DATA_POP
			let count = (byte & 0x3F) as u32 * 4 + 4;
			if TRACE_OPS {
				log_debug!("VSP += {:#x}*4+4 ({})", byte & 0x3F, count);
			}
			self.vsp += count;
			},
		0x4 ..= 0x7 => {	// ARM_EXIDX_CMD_DATA_PUSH
			let count = (byte & 0x3F) as u32 * 4 + 4;
			if TRACE_OPS {
				log_debug!("VSP -= {:#x}*4+4 ({})", byte & 0x3F, count);
			}
			self.vsp -= count;
			},
		0x8 => {	// ARM_EXIDX_CMD_REG_POP
			let extra = getb()?;
			if byte == 0x80 && extra == 0x00 {
				// Refuse to unwind
				return Err( Error::Refuse );
			}
			if TRACE_OPS {
				log_debug!("POP mask {:#x}{:02x}", byte & 0xF, extra);
			}
	
			// Lowest register at lowest stack
			if extra & 0x01 != 0 { self.regs[ 4] = self.pop()?; }	// R4
			if extra & 0x02 != 0 { self.regs[ 5] = self.pop()?; }	// R5
			if extra & 0x04 != 0 { self.regs[ 6] = self.pop()?; }	// R6
			if extra & 0x08 != 0 { self.regs[ 7] = self.pop()?; }	// R7
			if extra & 0x10 != 0 { self.regs[ 8] = self.pop()?; }	// R8
			if extra & 0x20 != 0 { self.regs[ 9] = self.pop()?; }	// R9
			if extra & 0x40 != 0 { self.regs[10] = self.pop()?; }	// R10
			if extra & 0x80 != 0 { self.regs[11] = self.pop()?; }	// R11
			if byte  &  0x1 != 0 { self.regs[12] = self.pop()?; }	// R12
			if byte  &  0x2 != 0 { self.regs[13] = self.pop()?; }	// R13
			if byte  &  0x4 != 0 { self.regs[14] = self.pop()?; }	// R14
			if byte  &  0x8 != 0 { self.regs[15] = self.pop()?; }	// R15
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
				self.regs[r] = self.pop()?;
			}
			if pop_lr { self.regs[14] = self.pop()?; }
			},
		0xB => match byte & 0xF
			{
			0 => return Ok(true),	// ARM_EXIDX_CMD_FINISH
			1 => {
				let extra = getb()?;
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
					if extra & 0x1 != 0 { self.regs[0] = self.pop()?; }	// R0
					if extra & 0x2 != 0 { self.regs[1] = self.pop()?; }	// R1
					if extra & 0x4 != 0 { self.regs[2] = self.pop()?; }	// R2
					if extra & 0x8 != 0 { self.regs[3] = self.pop()?; }	// R3
				}
				},
			2 => {	// vsp = vsp + 0x204 + (uleb128 << 2)
				let mut v = 0;
				loop {
					let b = getb()?;
					v <<= 7;
					v |= (b as u32) & 0x7F;
					if v & 0x80 == 0 {
						break;
					}
				}
				self.vsp += 0x204 + v * 4;
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
			if self.unwind_instr(b, || Self::getb(&mut it))? {
				break ;
			}
		}
		Ok( () )
	}
	pub fn unwind_long16(&mut self, instrs: u32, extra: &[u32]) -> Result<(), Error> {
		let mut it = WordBytesLE(instrs, 2).chain( extra.iter().flat_map(|w| WordBytesLE(*w, 4)) );
		while let Some(b) = it.next()
		{
			if self.unwind_instr(b, || Self::getb(&mut it))? {
				break ;
			}
		}
		Ok( () )
	}
	pub fn unwind_long32(&mut self, _instrs: u32, _extra: &[u32]) -> Result<(), Error> {
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


/// Look up unwind information for the given address
///
/// Returns: 
/// - Function (or unwind span?) base address
/// - Pointer to the unwind information slot (needed for relative lookups)
pub fn get_unwind_info_for(addr: usize) -> Option<(usize, &'static u32)>
{
	extern "C" {
		type Sym;
		static __exidx_start: [u32; 2];
		static __exidx_end: Sym;
	}

	// SAFE: Data at `__exidx_start` doesn't change
	let base = unsafe { &__exidx_start as *const _ as usize };
	// SAFE: 'static slice
	let exidx_tab: &[ [u32; 2] ] = unsafe { ::core::slice::from_raw_parts(&__exidx_start, (&__exidx_end as *const _ as usize - base) / (2*4)) };

	let mut best = (0,0);
	// Locate the closest entry before the return address
	for (i,e) in exidx_tab.iter().enumerate()
	{
		assert!(e[0] < 0x8000_0000);
		
		let fcn_start = usize::wrapping_add( e[0] as usize + 0x8000_0000, &e[0] as *const _ as usize );
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
