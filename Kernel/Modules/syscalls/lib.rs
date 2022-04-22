// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/mod.rs
//! Userland system-call interface
#![no_std]

#[allow(unused_imports)]
use kernel::prelude::*;
use kernel::memory::freeze::{Freeze,FreezeMut,FreezeError};

use args::Args;

#[macro_use]
extern crate kernel;
extern crate gui;
extern crate stack_dst;

mod objects;
mod args;

mod threads;
#[path="gui.rs"]
mod gui_calls;
mod vfs;
mod ipc_calls;
mod network_calls;

pub type ObjectHandle = u32;

#[derive(Debug)]
pub enum Error
{
	UnknownCall,
	TooManyArgs,
	BadValue,
	NoSuchObject(u32),
	TooManyObjects,
	InvalidBuffer(*const (), usize),
	BorrowFailure,
	MoveContention,
	InvalidUnicode(::core::str::Utf8Error),
}
impl From<::core::str::Utf8Error> for Error {
	fn from(v: ::core::str::Utf8Error) -> Self { Error::InvalidUnicode(v) }
}
impl From<FreezeError> for Error {
	fn from(_v: FreezeError) -> Self { Error::BorrowFailure }
}
impl ::core::fmt::Display for Error {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match *self
		{
		Error::UnknownCall => f.write_str("Unknown system call"),
		Error::TooManyArgs => f.write_str("Too many arguments needed"),
		Error::BadValue => f.write_str("Invalid value passed"),
		Error::NoSuchObject(v) => write!(f, "Object {} is invalid", v),
		Error::TooManyObjects => f.write_str("Out of object slots"),
		Error::InvalidBuffer(p,s) => write!(f, "Buffer {:p}+{} wasn't valid", p, s),
		Error::BorrowFailure => f.write_str("Contention on memory accesses"),
		Error::MoveContention => f.write_str("Contention on object transfer"),
		Error::InvalidUnicode(_) => f.write_str("Passed string wasn't valid unicode"),
		}
	}
}

/// Initialise PID0's handles
pub fn init(init_handle: ::kernel::vfs::handle::File) {
	vfs::init_handles(init_handle);
}

#[no_mangle]
/// Method called from architectue-specific (assembly) code
pub unsafe extern "C" fn syscalls_handler(id: u32, first_arg: *const usize, count: u32) -> u64
{
	let args = ::core::slice::from_raw_parts(first_arg, count as usize);
	//log_debug!("syscalls_handler({}, {:x?})", id, args);
	invoke(id, args)
}

fn invoke(call_id: u32, args: &[usize]) -> u64 {
	match invoke_int(call_id, &mut Args::new(args))
	{
	Ok(v) => v,
	Err(e) => {
		log_log!("Syscall formatting error in call {:#x} - {:?} {}", call_id, e, e);
		::kernel::threads::exit_process(0x8000_0000);
		// !0
		},
	}
}

fn error_code(value: u32) -> usize {
	value as usize + (!0 / 2)
}

/// Pack a result into a u32
/// Both `O` and `E` must be < 2^31
fn from_result<O: Into<u32>, E: Into<u32>>(r: Result<O,E>) -> u64 {
	match r
	{
	Ok(v) => {
		let v: u32 = v.into();
		assert!(v < 1<<31, "Result value {:#x} from {} is above 2^31", v, type_name!(O));
		v as u64
		}
	Err(e) => {
		let v: u32 = e.into();
		assert!(v < 1<<31, "Result value {:#x} from {} is above 2^31", v, type_name!(E));
		(1 << 31) | (v as u64)
		},
	}
}

use self::values::*;

#[path="../../../syscalls.inc.rs"]
mod values;

#[cfg(feature="native")]
pub mod native_exports {
	pub use crate::args::Args;
	pub mod values {
		pub use crate::values::*;
	}
	pub fn from_result<O: Into<u32>, E: Into<u32>>(r: Result<O,E>) -> u64 {
		crate::from_result(r)
	}
	pub fn get_file_handle(obj: u32) -> Result<::kernel::vfs::handle::File, crate::Error> {
		crate::vfs::get_file_handle(obj)
	}
	pub use crate::args::SyscallArg;
	
	pub use crate::objects::Object;
	pub use crate::objects::new_object;
	pub use crate::objects::give_object;
	pub use crate::objects::object_has_no_such_method_ref;
	pub use crate::objects::object_has_no_such_method_val;
}


#[inline(never)]
fn invoke_int(call_id: u32, args: &mut Args) -> Result<u64,Error>
{
	if call_id & 1 << 31 == 0
	{
		// Unbound system call
		// - Split using 15/16 into subsystems
		Ok(match call_id
		{
		// === 0: Threads and core
		// - 0/0: Userland log
		CORE_LOGWRITE => {
			let msg: Freeze<[u8]> = args.get()?;
			syscall_core_log(&msg); 0
			},
		// - Userland debug
		CORE_DBGVALUE => {
			let msg: Freeze<[u8]> = args.get()?;
			let val: usize = args.get()?;
			syscall_core_dbgvalue(&msg, val); 0
			},
		// - 0/2: Exit process
		CORE_EXITPROCESS => {
			let status: u32 = args.get()?;
			threads::exit(status); 0
			},
		CORE_TEXTINFO => {
			let group: u32 = args.get()?;
			let id: usize = args.get()?;
			let mut buf: FreezeMut<[u8]> = args.get()?;
			// TODO: Use a Result here
			syscall_core_textinfo(group, id, &mut buf) as u64
			},
		// - 0/2: Terminate current thread
		CORE_EXITTHREAD => {
			threads::terminate(); 0
			},
		// - 0/3: Start process
		CORE_STARTPROCESS => {
			let name: Freeze<str>  = args.get()?;
			let start: usize = args.get()?;
			let end  : usize = args.get()?;
			if start > end || end > ::kernel::arch::memory::addresses::USER_END {
				log_log!("CORE_STARTPROCESS - {:#x}--{:#x} invalid", start, end);
				return Err( Error::BadValue );
			}
			threads::newprocess(&name, start, end) as u64
			},
		// - 0/4: Start thread
		CORE_STARTTHREAD => {
			let ip: usize = args.get()?;
			let sp: usize = args.get()?;
			threads::newthread(sp, ip) as u64
			},
		// - 0/5: Wait for event
		CORE_WAIT => {
			let mut events: FreezeMut<[WaitItem]> = args.get()?;
			let timeout: u64 = args.get()?;
			threads::wait(&mut events, timeout)? as u64
			},
		CORE_FUTEX_SLEEP => {
			todo!("FUTEX_SLEEP");
			},
		CORE_FUTEX_WAKE => {
			todo!("FUTEX_SLEEP");
			},
		// === 1: Window Manager / GUI
		// - 1/0: New group (requires permission, has other restrictions)
		GUI_NEWGROUP => {
			let name: Freeze<str> = args.get()?;
			from_result(gui_calls::newgroup(&name))
			},
		// - 1/1: Bind group
		GUI_BINDGROUP => {
			let obj: u32 = args.get()?;
			if gui_calls::bind_group(obj)? {
				1
			}
			else {
				0
			}
			},
		// - 1/2: Clone group handle
		GUI_GETGROUP => {
			from_result(gui_calls::get_group())
			},
		// - 1/3: New window
		GUI_NEWWINDOW => {
			let name: Freeze<str> = args.get()?;
			from_result(gui_calls::newwindow(&name))
			},
		// === 2: Memory Mangement
		MEM_ALLOCATE => {
			let addr: usize = args.get()?;
			let count: usize = args.get()?;
			log_debug!("MEM_ALLOCATE({:#x},{})", addr, count);
			if addr & (::kernel::PAGE_SIZE-1) != 0 {
				return Err(Error::BadValue);
			}
			match ::kernel::memory::virt::allocate_user(addr as *mut (), count)
			{
			Ok(_) => 0,
			Err(e) => todo!("MEM_ALLOCATE - error {:?}", e),
			}
			},
		MEM_REPROTECT => {
			let addr: usize = args.get()?;
			let mode: u8 = args.get()?;
			log_debug!("MEM_REPROTECT({:#x},{})", addr, mode);
			if addr & (::kernel::PAGE_SIZE-1) != 0 {
				return Err(Error::BadValue);
			}
			let mode = match mode
				{
				0 => ::kernel::memory::virt::ProtectionMode::UserRO,
				1 => ::kernel::memory::virt::ProtectionMode::UserRW,
				2 => ::kernel::memory::virt::ProtectionMode::UserRX,
				3 => ::kernel::memory::virt::ProtectionMode::UserRWX,	// TODO: Should this be disallowed?
				_ => return Err( Error::BadValue ),
				};
			// SAFE: This internally does checks, but is marked as unsafe as a signal
			match unsafe { ::kernel::memory::virt::reprotect_user(addr as *mut (), mode) }
			{
			Ok( () ) => 0,
			Err( () ) => error_code(0) as u64,
			}
			},
		MEM_DEALLOCATE => {
			let addr: usize = args.get()?;
			// SAFE: This internally does checks, but is marked as unsafe as a signal
			match unsafe { ::kernel::memory::virt::reprotect_user(addr as *mut (), ::kernel::memory::virt::ProtectionMode::Unmapped) }
			{
			Ok( () ) => 0,
			Err( () ) => error_code(0) as u64,
			}
			},
		// === 3: IPC
		IPC_NEWPAIR => {
			match ipc_calls::new_pair()
			{
			Ok( (oh_a, oh_b) ) => oh_a as u64 | (oh_b as u64) << 32,
			Err( () ) => !0
			}
			},
		// === 4: Networking
		NET_CONNECT => {
			todo!("NET_CONNECT");
			},
		NET_LISTEN => {
			let local: crate::values::SocketAddress = { let p: Freeze<_> = args.get()?; *p };
			match network_calls::new_server(local)
			{
			Ok(v) => v as u64,
			Err(e) => e as u8 as u64,
			}
			},
		NET_BIND => {
			let local: crate::values::SocketAddress = { let p: Freeze<_> = args.get()?; *p };
			let remote: crate::values::MaskedSocketAddress = { let p: Freeze<_> = args.get()?; *p };
			match network_calls::new_free_socket(local, remote)
			{
			Ok(v) => v as u64,
			Err(e) => e as u8 as u64,
			}
			},
		// === *: Default
		_ => {
			log_error!("Unknown syscall {:05x}", call_id);
			!0
			},
		})
	}
	else
	{
		const CALL_MASK: u32 = 0x7FF;
		let handle_id = (call_id >> 0) & 0xFFFFF;
		let call_id = (call_id >> 20) & CALL_MASK;	// Call in upper part, as it's constant on user-side
		// Method call
		// - Look up the object (first argument) and dispatch using registered methods
		
		// - Call method
		match call_id as u16
		{
		0 ..= 0x3FD => {
			objects::call_object_ref(handle_id, call_id as u16, args)
			},
		crate::values::OBJECT_CLONE => {
			objects::clone_object(handle_id)
			},
		crate::values::OBJECT_GETCLASS => {
			objects::get_class(handle_id)
			},
		0x400 ..= 0x7FE => {
			// Call a method defined for the object class.
			objects::call_object_val(handle_id, call_id as u16, args)
			},
		//crate::values::OBJECT_FORGET => objects::forget_object(handle_id),
		crate::values::OBJECT_DROP => {
			// Destroy object
			objects::drop_object(handle_id);
			Ok(0)
			},
		_ => unreachable!(),
		}
	}
}

// TODO: Support a better user logging framework
#[inline(never)]
fn syscall_core_log(msg: &[u8]) {
	match ::core::str::from_utf8(msg)
	{
	Ok(v) => log_debug!("USER> {}", v),
	Err(e) =>
		log_debug!("USER [valid {}]> {:?}", e.valid_up_to(), ::kernel::lib::byte_str::ByteStr::new(msg)),
	}
}
#[inline(never)]
fn syscall_core_dbgvalue(msg: &[u8], val: usize) {
	log_debug!("USER DBG> {} {:#x}", ::core::str::from_utf8(msg).unwrap_or("BADUTF"), val);
}

#[inline(never)]
fn syscall_core_textinfo(group: u32, id: usize, buf: &mut [u8]) -> usize
{
	match group
	{
	crate::values::TEXTINFO_KERNEL => {
		let s = match id
			{
			0 => ::kernel::build_info::version_string(),
			1 => ::kernel::build_info::build_string(),
			_ => "",
			};
		let len = usize::min( s.len(), buf.len() );
		buf[..len].clone_from_slice(&s.as_bytes()[..len]);
		s.len()
		},
	_ => 0,
	}
}

