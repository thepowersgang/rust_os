// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/mod.rs
/// Userland system-call interface

/// Entrypoint invoked by the architecture-specific syscall handler
pub fn invoke(call_id: u32, mut args: &[usize]) -> u64
{
	if call_id & 1 << 31 == 0
	{
		// Unbound system call
		// - Split using 15/16 into subsystems
		match call_id
		{
		// === 0: Threads and core
		// - 0/0: Userland log
		0x0_0000 => {
			let msg = <&str>::get_arg(&mut args);
			syscall_core_log(msg); 0
			},
		// - 0/1: Commit userland log
		0x0_0001 => {
			syscall_core_logend(); 0
			},
		// - 0/2: Exit process
		0x0_0002 => {
			let status = <u32>::get_arg(&mut args);
			syscall_core_exit(status); 0
			},
		// - 0/3: Terminate current thread
		0x0_0003 => {
			syscall_core_terminate(); 0
			},
		// - 0/4: Start thread
		0x0_0004 => {
			let sp = <usize>::get_arg(&mut args);
			let ip = <usize>::get_arg(&mut args);
			syscall_core_newthread(sp, ip)
			},
		// - 0/5: Start process
		0x0_0005 => {
			todo!("Start process syscall");
			},
		// === 1: Window Manager / GUI
		// - 1/0: New group (requires permission, has other restrictions)
		0x1_0000 => {
			let name = <&str>::get_arg(&mut args);
			syscall_gui_newgroup(name)
			},
		// - 1/1: New window
		0x1_0001 => {
			let name = <&str>::get_arg(&mut args);
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
	}
}

type ObjectHandle = u64;

trait SyscallArg {
	fn get_arg(args: &mut &[usize]) -> Self;
}

impl<'a> SyscallArg for &'a str {
	fn get_arg(args: &mut &[usize]) -> Self {
		todo!("");
	}
}
impl SyscallArg for usize {
	fn get_arg(args: &mut &[usize]) -> Self {
		todo!("");
	}
}
impl SyscallArg for u32 {
	fn get_arg(args: &mut &[usize]) -> Self {
		todo!("");
	}
}

fn syscall_core_log(msg: &str) {
}
fn syscall_core_logend() {
}
fn syscall_core_exit(status: u32) {
}
fn syscall_core_terminate() {
}
fn syscall_core_newthread(sp: usize, ip: usize) -> ObjectHandle {
	todo!("");
}

fn syscall_gui_newgroup(name: &str) -> ObjectHandle {
	todo!("");
}
fn syscall_gui_newwindow(name: &str) -> ObjectHandle {
	todo!("");
}
