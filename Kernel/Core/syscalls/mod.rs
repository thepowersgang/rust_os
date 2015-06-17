// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/mod.rs
/// Userland system-call interface
use prelude::*;

/// Entrypoint invoked by the architecture-specific syscall handler
pub fn invoke(call_id: u32, args: &[usize]) -> u64 {
	invoke_int(call_id, args).unwrap_or(!0)
}
fn invoke_int(call_id: u32, mut args: &[usize]) -> Result<u64,()>
{
	Ok( if call_id & 1 << 31 == 0
	{
		// Unbound system call
		// - Split using 15/16 into subsystems
		match call_id
		{
		// === 0: Threads and core
		// - 0/0: Userland log
		0x0_0000 => {
			let msg = try!( <&str>::get_arg(&mut args) );
			syscall_core_log(msg); 0
			},
		// - 0/1: Commit userland log
		0x0_0001 => {
			syscall_core_logend(); 0
			},
		// - 0/2: Exit process
		0x0_0002 => {
			let status = try!( <u32>::get_arg(&mut args) );
			syscall_core_exit(status); 0
			},
		// - 0/3: Terminate current thread
		0x0_0003 => {
			syscall_core_terminate(); 0
			},
		// - 0/4: Start thread
		0x0_0004 => {
			let sp = try!( <usize>::get_arg(&mut args) );
			let ip = try!( <usize>::get_arg(&mut args) );
			syscall_core_newthread(sp, ip)
			},
		// - 0/5: Start process
		0x0_0005 => {
			todo!("Start process syscall");
			},
		// === 1: Window Manager / GUI
		// - 1/0: New group (requires permission, has other restrictions)
		0x1_0000 => {
			let name = try!( <&str>::get_arg(&mut args) );
			syscall_gui_newgroup(name)
			},
		// - 1/1: New window
		0x1_0001 => {
			let name = try!( <&str>::get_arg(&mut args) );
			syscall_gui_newwindow(name)
			},
		_ => {
			log_error!("Unknown syscall {:05x}", call_id);
			0
			},
		}
	}
	else
	{
		let handle_id = (call_id >> 16) & 0x7FFF;
		let call_id = call_id & 0xFFFF;
		// Method call
		// - Look up the object (first argument) and dispatch using registered methods
		if call_id == 0xFFFF {
			// Destroy object
		}
		else {
			// Call a method defined for the object class?
		}
		todo!("");
	})
}

type ObjectHandle = u64;

trait SyscallArg {
	fn get_arg(args: &mut &[usize]) -> Result<Self,()>;
}

impl<'a> SyscallArg for &'a str {
	fn get_arg(args: &mut &[usize]) -> Result<Self,()> {
		if args.len() < 2 {
			return Err( () );
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
				return Err( () );
			} };
		match ::core::str::from_utf8(bs)
		{
		Ok(v) => Ok(v),
		Err(_) => return Err( () ),
		}
	}
}
impl SyscallArg for usize {
	fn get_arg(args: &mut &[usize]) -> Result<Self,()> {
		if args.len() < 1 {
			return Err( () );
		}
		let rv = args[0];
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for u32 {
	fn get_arg(args: &mut &[usize]) -> Result<Self,()> {
		if args.len() < 1 {
			return Err( () );
		}
		let rv = args[0] as u32;
		*args = &args[1..];
		Ok( rv )
	}
}

fn syscall_core_log(msg: &str) {
	todo!("syscall_core_log(msg={})", msg);
}
fn syscall_core_logend() {
	todo!("syscall_core_logend()");
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
