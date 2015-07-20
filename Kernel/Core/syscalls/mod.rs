// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/mod.rs
/// Userland system-call interface
use prelude::*;

mod objects;
mod gui;
mod vfs;

pub type ObjectHandle = u32;

#[allow(raw_pointer_derive)]
#[derive(Debug)]
pub enum Error
{
	TooManyArgs,
	BadValue,
    NoSuchObject,
	InvalidBuffer(*const (), usize),
	InvalidUnicode(::core::str::Utf8Error),
}
impl From<::core::str::Utf8Error> for Error {
	fn from(v: ::core::str::Utf8Error) -> Self { Error::InvalidUnicode(v) }
}

/// Entrypoint invoked by the architecture-specific syscall handler
pub fn invoke(call_id: u32, args: &[usize]) -> u64 {
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
			let msg = try!( <&str>::get_arg(&mut args) );
			syscall_core_log(msg); 0
			},
		// - 0/1: Exit process
		CORE_EXITPROCESS => {
			let status = try!( <u32>::get_arg(&mut args) );
			syscall_core_exit(status); 0
			},
		// - 0/2: Terminate current thread
		CORE_EXITTHREAD => {
			syscall_core_terminate(); 0
			},
		// - 0/3: Start process
		CORE_STARTPROCESS => {
			let ip = try!( <usize>::get_arg(&mut args) );
			let sp = try!( <usize>::get_arg(&mut args) );
			let start = try!( <usize>::get_arg(&mut args) );
			let end   = try!( <usize>::get_arg(&mut args) );
			if start > end || end > ::arch::memory::addresses::USER_END {
				return Err( Error::BadValue );
			}
			syscall_core_newprocess(ip, sp, start, end) as u64
			},
		// - 0/4: Start thread
		CORE_STARTTHREAD => {
			let ip = try!( <usize>::get_arg(&mut args) );
			let sp = try!( <usize>::get_arg(&mut args) );
			syscall_core_newthread(sp, ip) as u64
			},
		// - 0/5: Wait for event
		CORE_WAIT => {
			let events = try!( <&[WaitItem]>::get_arg(&mut args) );
			let timeout = try!( <u64>::get_arg(&mut args) );
			try!(syscall_core_wait(events, timeout)) as u64
			},
		// === 1: Window Manager / GUI
		// - 1/0: New group (requires permission, has other restrictions)
		GUI_NEWGROUP => {
			let name = try!( <&str>::get_arg(&mut args) );
			from_result(gui::newgroup(name))
			},
		// - 1/1: New window
		GUI_NEWWINDOW => {
			let name = try!( <&str>::get_arg(&mut args) );
			from_result(gui::newwindow(name))
			},
		// === 2: VFS
		// - 2/0: Open node (for stat)
		VFS_OPENNODE => {
			todo!("VFS_OPENNODE");
			},
		// - 2/1: Open file
		VFS_OPENFILE => {
			let name = try!( <&[u8]>::get_arg(&mut args) );
			let mode = try!( <u32>::get_arg(&mut args) );
			from_result( vfs::openfile(name, mode) )
			},
		// - 2/2: Open directory
		VFS_OPENDIR => {
			todo!("VFS_OPENDIR");
			},
		// === 3: Memory Mangement
		MEM_ALLOCATE => {
			let addr = try!(<usize>::get_arg(&mut args));
			let count = try!(<usize>::get_arg(&mut args));
			// Wait? Why do I have a 'mode' here?
			log_debug!("MEM_ALLOCATE({:#x},{})", addr, count);
			::memory::virt::allocate_user(addr as *mut (), count); 0
			//match ::memory::virt::allocate_user(addr as *mut (), count)
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
				0 => ::memory::virt::ProtectionMode::UserRO,
				1 => ::memory::virt::ProtectionMode::UserRW,
				2 => ::memory::virt::ProtectionMode::UserRX,
				3 => ::memory::virt::ProtectionMode::UserRWX,	// TODO: Should this be disallowed?
				_ => return Err( Error::BadValue ),
				};
			// SAFE: This internally does checks, but is marked as unsafe as a signal
			match unsafe { ::memory::virt::reprotect_user(addr as *mut (), mode) }
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

trait SyscallArg {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error>;
}

impl<'a> SyscallArg for &'a str {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 2 {
			return Err( Error::TooManyArgs );
		}
		let ptr = args[0] as *const u8;
		let len = args[1];
		*args = &args[2..];
		// TODO: Freeze the page to prevent the user from messing with it
		// SAFE: (uncheckable) lifetime of result should really be 'args, but can't enforce that
		let bs = unsafe {
			if let Some(v) = ::memory::buf_to_slice(ptr, len) {	
				v
			}
			else {
				return Err( Error::InvalidBuffer(ptr as *const (), len) );
			} };
		
		Ok(try!( ::core::str::from_utf8(bs) ))
	}
}
macro_rules! def_slice_get_arg {
	($t:ty) => {
		impl<'a> SyscallArg for &'a [$t] {
			fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
				if args.len() < 2 {
					return Err( Error::TooManyArgs );
				}
				let ptr = args[0] as *const $t;
				let len = args[1];
				*args = &args[2..];
				// TODO: Freeze the page to prevent the user from messing with it
				// SAFE: (uncheckable) lifetime of result should really be 'args, but can't enforce that
				unsafe {
					if let Some(v) = ::memory::buf_to_slice(ptr, len) {	
						Ok(v)
					}
					else {
						Err( Error::InvalidBuffer(ptr as *const (), len) )
					}
				}
			}
		}
	};
}
def_slice_get_arg!{u8}
def_slice_get_arg!{u32}
def_slice_get_arg!{WaitItem}

impl<'a> SyscallArg for &'a mut [u8] {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 2 {
			return Err( Error::TooManyArgs );
		}
		let ptr = args[0] as *mut u8;
		let len = args[1];
		*args = &args[2..];
		// TODO: Freeze the page to prevent the user from messing with it
		// SAFE: (uncheckable) lifetime of result should really be 'args, but can't enforce that
		unsafe {
			if let Some(v) = ::memory::buf_to_slice_mut(ptr, len) {
				Ok(v)
			}
			else {
				Err( Error::InvalidBuffer(ptr as *const (), len) )
			}
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

fn syscall_core_log(msg: &str) {
	log_debug!("syscall_core_log - {}", msg);
}
fn syscall_core_exit(status: u32) {
	todo!("syscall_core_exit(status={:x})", status);
}
fn syscall_core_terminate() {
	todo!("syscall_core_terminate()");
}
fn syscall_core_newthread(sp: usize, ip: usize) -> ObjectHandle {
	todo!("syscall_core_newthread(sp={:#x},ip={:#x})", sp, ip);
}
fn syscall_core_newprocess(ip: usize, sp: usize, clone_start: usize, clone_end: usize) -> ObjectHandle {
	// 1. Create a new process image (virtual address space)
	let mut process = ::threads::ProcessHandle::new("TODO", clone_start, clone_end);
	// 3. Create a new thread using that process image with the specified ip/sp
	process.start_root_thread(ip, sp);
	
	struct Process(::threads::ProcessHandle);
	impl objects::Object for Process {
		fn handle_syscall(&self, call: u16, _args: &[usize]) -> Result<u64,Error> {
			match call
			{
			_ => todo!("Process::handle_syscall({}, ...)", call),
			}
		}
        fn bind_wait(&self, _flags: u32, _obj: &mut ::threads::SleepObject) -> u32 { 0 }
        fn clear_wait(&self, _flags: u32, _obj: &mut ::threads::SleepObject) -> u32 { 0 }
	}

	objects::new_object( Process(process) )
}

// ret: number of events triggered
fn syscall_core_wait(events: &[WaitItem], wake_time_mono: u64) -> Result<u32,Error>
{
    let mut waiter = ::threads::SleepObject::new("syscall_core_wait");
    let mut num_bound = 0;
    for ev in events {
        num_bound += try!(objects::wait_on_object(ev.object, ev.flags, &mut waiter));
    }

    if num_bound == 0 && wake_time_mono == !0 {
        // Attempting to sleep on no events with an infinite timeout! Would sleep forever
        todo!("What to do when a thread tries to sleep forever");
    }

    // A wake time of 0 means to not sleep at all, just check the status of the events
    // TODO: There should be a more efficient way of doing this, than binding only to unbind again
    if wake_time_mono != 0 {
        // !0 indicates an unbounded wait (no need to set a wakeup time)
        if wake_time_mono != !0 {
            todo!("Set a wakeup timer at {}", wake_time_mono);
        }
        waiter.wait();
    }

    Ok( events.iter().fold(0, |total,ev| total + objects::clear_wait(ev.object, ev.flags, &mut waiter).unwrap()) )
}

