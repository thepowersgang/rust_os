// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/mod.rs
//! Userland system-call interface
#![feature(no_std)]
#![no_std]
#![feature(associated_consts)]
#![feature(core_slice_ext,core_str_ext)]
#![feature(reflect_marker)]

#[macro_use]
extern crate kernel;

extern crate gui;

extern crate stack_dst;

#[allow(unused_imports)]
use kernel::prelude::*;

use kernel::memory::freeze::{Freeze,FreezeMut,FreezeError};

mod objects;

mod threads;
#[path="gui.rs"]
mod gui_calls;
mod vfs;

pub type ObjectHandle = u32;

#[allow(raw_pointer_derive)]
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
	InvalidUnicode(::core::str::Utf8Error),
}
impl From<::core::str::Utf8Error> for Error {
	fn from(v: ::core::str::Utf8Error) -> Self { Error::InvalidUnicode(v) }
}
impl From<FreezeError> for Error {
	fn from(_v: FreezeError) -> Self { Error::BorrowFailure }
}

#[no_mangle]
pub unsafe extern "C" fn syscalls_handler(id: u32, first_arg: *const usize, count: u32) -> u64
{
	//log_debug!("syscalls_handler({}, {:p}+{})", id, first_arg, count);
	invoke(id, ::core::slice::from_raw_parts(first_arg, count as usize))
}

/// Entrypoint invoked by the architecture-specific syscall handler
fn invoke(call_id: u32, args: &[usize]) -> u64 {
	match invoke_int(call_id, args)
	{
	Ok(v) => v,
	Err(e) => {
		log_log!("Syscall formatting error in call {:#x} - {:?}", call_id, e);
		!0
		},
	}
}

fn error_code(value: u32) -> usize {
	value as usize + usize::max_value() / 2
}

/// Pack a result into a u32
/// Both `O` and `E` must be < 2^31
fn from_result<O: Into<u32>, E: Into<u32>>(r: Result<O,E>) -> u64 {
	match r
	{
	Ok(v) => {
		let v: u32 = v.into();
		assert!(v < 1<<31);
		v as u64
		}
	Err(e) => {
		let v: u32 = e.into();
		assert!(v < 1<<31);
		(1 << 31) | (v as u64)
		},
	}
}

use self::values::*;
#[path="../../../syscalls.inc.rs"]
mod values;

#[inline(never)]
fn invoke_int(call_id: u32, mut args: &[usize]) -> Result<u64,Error>
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
			let msg = try!( <Freeze<str>>::get_arg(&mut args) );
			syscall_core_log(&msg); 0
			},
		CORE_TEXTINFO => {
			let group = try!( <u32>::get_arg(&mut args) );
			let id = try!( <usize>::get_arg(&mut args) );
			let mut buf = try!( <FreezeMut<[u8]>>::get_arg(&mut args) );
			// TODO: Use a Result here
			syscall_core_textinfo(group, id, &mut buf) as u64
			},
		// - 0/1: Exit process
		CORE_EXITPROCESS => {
			let status = try!( <u32>::get_arg(&mut args) );
			threads::exit(status); 0
			},
		// - 0/2: Terminate current thread
		CORE_EXITTHREAD => {
			threads::terminate(); 0
			},
		// - 0/3: Start process
		CORE_STARTPROCESS => {
			let ip = try!( <usize>::get_arg(&mut args) );
			let sp = try!( <usize>::get_arg(&mut args) );
			let start = try!( <usize>::get_arg(&mut args) );
			let end   = try!( <usize>::get_arg(&mut args) );
			if start > end || end > ::kernel::arch::memory::addresses::USER_END {
				log_log!("CORE_STARTPROCESS - {:#x}--{:#x} invalid", start, end);
				return Err( Error::BadValue );
			}
			threads::newprocess("TODO", ip, sp, start, end) as u64
			},
		// - 0/4: Start thread
		CORE_STARTTHREAD => {
			let ip = try!( <usize>::get_arg(&mut args) );
			let sp = try!( <usize>::get_arg(&mut args) );
			threads::newthread(sp, ip) as u64
			},
		// - 0/5: Wait for event
		CORE_WAIT => {
			let mut events = try!( <FreezeMut<[WaitItem]>>::get_arg(&mut args) );
			let timeout = try!( <u64>::get_arg(&mut args) );
			try!(threads::wait(&mut events, timeout)) as u64
			},
		// === 1: Window Manager / GUI
		// - 1/0: New group (requires permission, has other restrictions)
		GUI_NEWGROUP => {
			let name = try!( <Freeze<str>>::get_arg(&mut args) );
			from_result(gui_calls::newgroup(&name))
			},
		// - 1/1: Bind group
		GUI_BINDGROUP => {
			let obj = try!( <u32>::get_arg(&mut args) );
			if try!(gui_calls::bind_group(obj)) {
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
			let name = try!( <Freeze<str>>::get_arg(&mut args) );
			from_result(gui_calls::newwindow(&name))
			},
		// === 2: VFS
		// - 2/0: Open node (for stat)
		VFS_OPENNODE => {
			let name = try!( <Freeze<[u8]>>::get_arg(&mut args) );
			from_result( vfs::opennode(&name) )
			},
		// - 2/1: Open file
		VFS_OPENFILE => {
			let name = try!( <Freeze<[u8]>>::get_arg(&mut args) );
			let mode = try!( <u8>::get_arg(&mut args) );
			from_result( vfs::openfile(&name, mode) )
			},
		// - 2/2: Open directory
		VFS_OPENDIR => {
			let name = try!( <Freeze<[u8]>>::get_arg(&mut args) );
			from_result( vfs::opendir(&name) )
			},
		// - 2/3: Open directory
		VFS_OPENLINK => {
			let name = try!( <Freeze<[u8]>>::get_arg(&mut args) );
			from_result( vfs::openlink(&name) )
			},
		// === 3: Memory Mangement
		MEM_ALLOCATE => {
			let addr = try!(<usize>::get_arg(&mut args));
			let count = try!(<usize>::get_arg(&mut args));
			// Wait? Why do I have a 'mode' here?
			log_debug!("MEM_ALLOCATE({:#x},{})", addr, count);
			::kernel::memory::virt::allocate_user(addr as *mut (), count); 0
			//match ::kernel::memory::virt::allocate_user(addr as *mut (), count)
			//{
			//Ok(_) => 0,
			//Err(e) => todo!("MEM_ALLOCATE - error {:?}", e),
			//}
			},
		MEM_REPROTECT => {
			let addr = try!(<usize>::get_arg(&mut args));
			let mode = try!(<u8>::get_arg(&mut args));
			log_debug!("MEM_REPROTECT({:#x},{})", addr, mode);
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
			let addr = try!(<usize>::get_arg(&mut args));
			todo!("MEM_DEALLOCATE({:#x})", addr)
			},
		// === *: Default
		_ => {
			log_error!("Unknown syscall {:05x}", call_id);
			0
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
		if call_id == CALL_MASK {
			// Destroy object
			objects::drop_object(handle_id);
			Ok(0)
		}
		else {
			// Call a method defined for the object class?
			objects::call_object(handle_id, call_id as u16, args)
		}
	}
}

trait SyscallArg: Sized {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error>;
}

// POD - Plain Old Data
pub trait Pod { }
impl Pod for u8 {}
impl Pod for u32 {}
impl Pod for values::WaitItem {}
impl Pod for values::GuiEvent {}	// Kinda lies, but meh

impl<T: Pod> SyscallArg for Freeze<[T]>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 2 {
			return Err( Error::TooManyArgs );
		}
		let ptr = args[0] as *const T;
		let len = args[1];
		*args = &args[2..];
		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe {
			// 1. Check if the pointer is into user memory
			// TODO: ^^^
			// 2. Ensure that the pointed slice is valid (overlaps checks by Freeze, but gives a better error)
			// TODO: Replace this check with mapping FreezeError
			let bs = if let Some(v) = ::kernel::memory::buf_to_slice(ptr, len) {
					v
				} else {
					return Err( Error::InvalidBuffer(ptr as *const (), len) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( try!(Freeze::new(bs)) )
		}
	}
}
impl SyscallArg for Freeze<str> {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		let ret = try!(Freeze::<[u8]>::get_arg(args));
		// SAFE: Transmuting [u8] to str is valid if the str is valid UTF-8
		unsafe { 
			try!( ::core::str::from_utf8(&ret) );
			Ok(::core::mem::transmute(ret))
		}
	}
}
impl<T: Pod> SyscallArg for FreezeMut<T>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 1 {
			return Err( Error::TooManyArgs );
		}
		let ptr = args[0] as *mut T;

		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe { 
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( try!(FreezeMut::new(&mut *ptr)) )
		}
	}
}
impl<T: Pod> SyscallArg for FreezeMut<[T]>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 2 {
			return Err( Error::TooManyArgs );
		}
		let ptr = args[0] as *mut T;
		let len = args[1];
		*args = &args[2..];
		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe { 
			// 1. Check if the pointer is into user memory
			// TODO: ^^^
			// 2. Ensure that the pointed slice is valid (overlaps checks by Freeze, but gives a better error)
			// TODO: Replace this check with mapping FreezeError
			let bs =  if let Some(v) = ::kernel::memory::buf_to_slice_mut(ptr, len) {	
					v
				} else {
					return Err( Error::InvalidBuffer(ptr as *const (), len) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( try!(FreezeMut::new(bs)) )
		}
	}
}

impl SyscallArg for usize {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 1 {
			return Err( Error::TooManyArgs );
		}
		let rv = args[0];
		*args = &args[1..];
		Ok( rv )
	}
}
#[cfg(target_pointer_width="64")]
impl SyscallArg for u64 {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 1 {
			return Err( Error::TooManyArgs );
		}
		let rv = args[0] as u64;
		*args = &args[1..];
		Ok( rv )
	}
}
#[cfg(target_pointer_width="32")]
impl SyscallArg for u64 {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 2 {
			return Err( Error::TooManyArgs );
		}
		let rv = args[0] as u64 | (args[1] as u64) << 32;
		*args = &args[2..];
		Ok( rv )
	}
}

impl SyscallArg for u32 {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 1 {
			return Err( Error::TooManyArgs );
		}
		let rv = args[0] as u32;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for u16 {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 1 {
			return Err( Error::TooManyArgs );
		}
		let rv = args[0] as u16;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for u8 {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 1 {
			return Err( Error::TooManyArgs );
		}
		let rv = args[0] as u8;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for bool {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 1 {
			return Err( Error::TooManyArgs );
		}
		let rv = (args[0] as u8) != 0;
		*args = &args[1..];
		Ok( rv )
	}
}

// TODO: Support a better user logging framework
#[inline(never)]
fn syscall_core_log(msg: &str) {
	log_debug!("USER> {}", msg);
}

#[inline(never)]
fn syscall_core_textinfo(group: u32, id: usize, buf: &mut [u8]) -> usize
{
	match group
	{
	::values::TEXTINFO_KERNEL =>
		match id
		{
		0 => { buf.clone_from_slice( ::kernel::VERSION_STRING.as_bytes() ); ::kernel::VERSION_STRING.len() },
		1 => { buf.clone_from_slice( ::kernel::BUILD_STRING.as_bytes() ); ::kernel::BUILD_STRING.len() },
		_ => 0,
		},
	_ => 0,
	}
}

