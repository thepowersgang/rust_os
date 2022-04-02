
mod inner {
	#![allow(dead_code)]

	#[allow(non_camel_case_types)]
	pub struct sbiret {
		error: isize,
		val: isize,
	}
	impl sbiret {
		pub fn to_result(self) -> Result<isize,Error> {
			match self.error {
			0 => Ok(self.val),
			-1 => Err(Error::Failed),
			-2 => Err(Error::NotSupported),
			-3 => Err(Error::InvalidParam),
			-4 => Err(Error::Denied),
			-5 => Err(Error::InvalidAddress),
			-6 => Err(Error::AlreadyAvailable),
			-7 => Err(Error::AlreadyStarted),
			-8 => Err(Error::AlreadyStopped),
			_ => Err(Error::Unknown(self.error)),
			}
		}
	}
	#[derive(Debug)]
	pub enum Error {
		Failed,
		NotSupported,
		InvalidParam,
		Denied,
		InvalidAddress,
		AlreadyAvailable,
		AlreadyStarted,
		AlreadyStopped,
		Unknown(isize),
	}

	macro_rules! call_sbi {
		($eid:literal,$fid:literal : ($($arg:ident),*)) => { call_sbi!($eid,$fid :($($arg),*) options()) };
		($eid:literal,$fid:literal : ($($arg:ident),*) options($($opts:tt)*)) => { call_sbi!(@a (in("a7") $eid, in("a6") $fid,) () ($($arg)*) ($($opts)*)) };

		(@a ( $($cur:tt)* ) (   ) ($arg:ident $($rest:ident)*) ($($opts:tt)*)) => { call_sbi!(@a ($($cur)* in("a0") $arg,) (,   ) ($($rest)*) ($($opts)*)) };
		(@a ( $($cur:tt)* ) (,  ) ($arg:ident $($rest:ident)*) ($($opts:tt)*)) => { call_sbi!(@a ($($cur)* in("a1") $arg,) (,,  ) ($($rest)*) ($($opts)*)) };
		(@a ( $($cur:tt)* ) (,, ) ($arg:ident $($rest:ident)*) ($($opts:tt)*)) => { call_sbi!(@a ($($cur)* in("a2") $arg,) (,,, ) ($($rest)*) ($($opts)*)) };
		(@a ( $($cur:tt)* ) (,,,) ($arg:ident $($rest:ident)*) ($($opts:tt)*)) => { call_sbi!(@a ($($cur)* in("a3") $arg,) (,,,,) ($($rest)*) ($($opts)*)) };

		(@a ( $($cur:tt)* ) ($($commas:tt)*) () ()) => {{
			let (error, val);
			::core::arch::asm!("ecall",
				$($cur)*
				lateout("a0") error, lateout("a1") val,
				lateout("a2") _, lateout("a3") _,
				lateout("a4") _, lateout("a5") _
				);
			sbiret { error, val }
			}};
		(@a ( $($cur:tt)* ) ($($commas:tt)*) () (noreturn)) => {{
			::core::arch::asm!("ecall", $($cur)* options(noreturn))
			}};
	}
	macro_rules! extern_sbi {
		( $eid:literal : $( $vis:vis fn $name:ident($($arg:ident: $ty:ty),*) = $fid:literal $($opt:ident)*;)* ) => {
			$(
			$vis unsafe fn $name($($arg:$ty),*) -> sbiret {
				call_sbi!( $eid,$fid : ($($arg),*) options($($opt)*) )
			}
			)*
		};
	}

	// Legacy
	//extern_sbi!{0x00: pub fn sbi_set_timer(stime_value: u64) = 0};
	// Base Extension
	extern_sbi!{ 0x10:
		pub fn sbi_get_spec_version() = 0;
		pub fn sbi_get_impl_id() = 1;
		pub fn sbi_get_impl_version() = 2;
		pub fn sbi_probe_extension(extension_id: isize) = 3;
		pub fn sbi_get_mvendorid() = 4;
		pub fn sbi_get_marchid() = 5;
		pub fn sbi_get_mimpid() = 6;
	}

	// "Hart State Management"
	extern_sbi!{ 0x48534D:
		pub fn sbi_hart_start(hartid: usize, start_addr: usize, opaque: usize) = 0;
		pub fn sbi_hart_stop() = 1 noreturn;
		pub fn sbi_hart_get_status(hartid: usize) = 2;
		pub fn sbi_hart_suspend(suspend_ty: u32, resume_addr: usize, opaque: usize) = 3;
	}
	#[repr(isize)]
	#[allow(non_camel_case_types)]
	pub enum HartState {
		STARTED         = 0,
		STOPPED         = 1,
		START_PENDING   = 2,
		STOP_PENDING    = 3,
		SUSPENDED       = 4,
		SUSPEND_PENDING = 5,
		RESUME_PENDING  = 6,
	}
	impl ::core::convert::TryFrom<isize> for HartState {
		type Error = isize;
		fn try_from(v: isize) -> Result<HartState,isize> {
			if 0 <= v && v <= 6 {
				// SAFE: Range checked
				Ok(unsafe { ::core::mem::transmute(v) })
			}
			else {
				Err(v)
			}
		}
	}
}
use self::inner::*;

pub fn dump_sbi_info()
{
	macro_rules! try_call {
		($name:ident) => { match $name().to_result() { Ok(v) => v, Err(e) => { log_warning!("dump_sbi_info: {} {:?}", stringify!($name), e); return; } } };
	}
	// SAFE: All FFI functions here have no side-effects
	unsafe {
		log_log!("SBI v{:#x}", try_call!(sbi_get_spec_version));
		let impl_id = try_call!(sbi_get_impl_id);
		let impl_name = match impl_id
			{
			0 => "BBL",
			1 => "OpenSBI",
			2 => "Xvisor",
			3 => "KVM",
			4 => "RustSBI",
			5 => "Diosix",
			_ => "?",
			};
		log_log!("Vendor: {} ({}) v{}", impl_name, impl_id, try_call!(sbi_get_impl_version));
		log_log!("Machine: vendor={:#x} arch={:#x} imp={:#x}",
			try_call!(sbi_get_mvendorid),
			try_call!(sbi_get_marchid),
			try_call!(sbi_get_mimpid),
			);
	}
}

