
/// Quick macro to make this code look similar to the kernel
macro_rules! log_error {
	($s:tt $($v:tt)*) => {
		kernel_log!(concat!("Error: ",$s) $($v)*)
		};
}
macro_rules! log_warning {
	($s:tt $($v:tt)*) => {
		kernel_log!(concat!("Warning: ",$s) $($v)*)
		};
}
macro_rules! log_debug {
	($s:expr, $($e:expr),*) => {{
		static mut S_DBG_BUF: ::syscalls::logging::FixedBuf = ::syscalls::logging::FixedBuf::new();
		use core::fmt::Write;
		// SAFE: Assuming we don't end up with multiple threads doing a backtrace
		let _ = write!(&mut ::syscalls::logging::ThreadLogWriter::new(unsafe { &mut S_DBG_BUF }), concat!("Debug: ",$s), $($e),*);
		}
		};
}
#[allow(dead_code)]	// Internally defines functions that we don't use
#[path="../../Kernel/Core/arch/armv7/aeabi_unwind.rs"]
mod aeabi_unwind;


pub struct Backtrace(aeabi_unwind::UnwindState);
impl Backtrace {
	pub fn new() -> Backtrace {
		let rs = aeabi_unwind::UnwindState::new_cur();
		Backtrace(rs)
	}
}

impl ::core::fmt::Debug for Backtrace {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let mut rs = self.0.clone();
		let mut addr = rs.get_ip() as usize;
		try!(write!(f, "{:#x}", addr));
		while let Some(info) = aeabi_unwind::get_unwind_info_for(addr)
		{
			// - Subtract 1 to avoid 'bl' at the end of a function tricking the resolution
			//match ::symbols::get_symbol_for_addr(addr-1) {
			//Some( (name,ofs) ) => try!(write!(f, " > {:#x} {}+{:#x}", addr, ::symbols::Demangle(name), ofs+1)),
			//None => try!(write!(f, " > {:#x}", addr));,
			//}
			match rs.unwind_step(info.1)
			{
			Ok(_) => {},
			Err(e) => return write!(f, " > Error {:?}", e),
			}
			let new_addr = rs.get_lr() as usize;
			if addr == new_addr {
				return write!(f, " > Same stack frame detected {:#x}", addr);
			}
			addr = new_addr;
			try!(write!(f, " > {:#x}", addr));
		}
		Ok( () )
	}
}

