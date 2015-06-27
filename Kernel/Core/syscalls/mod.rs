// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/mod.rs
/// Userland system-call interface
use prelude::*;

mod objects;

#[allow(raw_pointer_derive)]
#[derive(Debug)]
enum Error
{
	TooManyArgs,
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

use self::values::*;
#[path="../../../syscalls.inc.rs"]
mod values;

fn invoke_int(call_id: u32, mut args: &[usize]) -> Result<u64,Error>
{
	Ok( if call_id & 1 << 31 == 0
	{
		// Unbound system call
		// - Split using 15/16 into subsystems
		match call_id
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
		// - 0/3: Start thread
		CORE_STARTTHREAD => {
			let sp = try!( <usize>::get_arg(&mut args) );
			let ip = try!( <usize>::get_arg(&mut args) );
			syscall_core_newthread(sp, ip) as u64
			},
		// - 0/4: Start process
		CORE_STARTPROCESS => {
			todo!("Start process syscall");
			},
		// === 1: Window Manager / GUI
		// - 1/0: New group (requires permission, has other restrictions)
		GUI_NEWGROUP => {
			let name = try!( <&str>::get_arg(&mut args) );
			syscall_gui_newgroup(name) as u64
			},
		// - 1/1: New window
		GUI_NEWWINDOW => {
			let name = try!( <&str>::get_arg(&mut args) );
			syscall_gui_newwindow(name) as u64
			},
		// === 2: VFS
		// - 2/0: Open node (for stat)
		VFS_OPENNODE => {
			todo!("VFS_OPEN");
			},
		// - 2/1: Open file
		VFS_OPENFILE => {
			let name = try!( <&[u8]>::get_arg(&mut args) );
			let mode = try!( <u32>::get_arg(&mut args) );
			(match syscall_vfs_openfile(name, mode)
			{
			Ok(v) => v,
			Err(v) => (1<<31)|v,
			} as u64)
			},
		// - 2/2: Open directory
		VFS_OPENDIR => {
			todo!("VFS_OPENDIR");
			},
		// === *: Default
		_ => {
			log_error!("Unknown syscall {:05x}", call_id);
			0
			},
		}
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
			objects::drop_object(handle_id); 0
		}
		else {
			// Call a method defined for the object class?
			objects::call_object(handle_id, call_id as u16, args)
		}
	})
}

type ObjectHandle = u32;

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
impl<'a> SyscallArg for &'a [u8] {
	fn get_arg(args: &mut &[usize]) -> Result<Self,Error> {
		if args.len() < 2 {
			return Err( Error::TooManyArgs );
		}
		let ptr = args[0] as *const u8;
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

fn syscall_core_log(msg: &str) {
	log_debug!("syscall_core_log - {}", msg);
}
fn syscall_core_exit(status: u32) {
	todo!("syscall_core_exit(status={})", status);
}
fn syscall_core_terminate() {
	todo!("syscall_core_terminate()");
}
fn syscall_core_newthread(sp: usize, ip: usize) -> ObjectHandle {
	todo!("syscall_core_newthread(sp={:#x},ip={:#x})", sp, ip);
}

fn syscall_gui_newgroup(name: &str) -> ObjectHandle {
	todo!("syscall_gui_newgroup(name={})", name);
}
fn syscall_gui_newwindow(name: &str) -> ObjectHandle {
	todo!("syscall_gui_newwindow(name={})", name);
}

fn syscall_vfs_openfile(path: &[u8], mode: u32) -> Result<ObjectHandle,u32> {
	struct File(::vfs::handle::File);

	impl objects::Object for File {
		fn handle_syscall(&self, call: u16, args: &[usize]) -> u64 {
			todo!("File::handle_syscall({}, ...)", call);
		}
	}
	
	let mode = match mode
		{
		1 => ::vfs::handle::FileOpenMode::SharedRO,
		2 => ::vfs::handle::FileOpenMode::Execute,
		_ => todo!("Unkown mode {:x}", mode),
		};
	match ::vfs::handle::File::open(::vfs::Path::new(path), mode)
	{
	Ok(h) => Ok( objects::new_object( File(h) ) ),
	Err(e) => todo!("syscall_vfs_openfile - e={:?}", e),
	}
}

