// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/syscalls/threads.rs
//! Thread management calls

use kernel::prelude::*;

use ObjectHandle;
use Error;
use values;
use SyscallArg;
use kernel::memory::freeze::FreezeMut;
//use kernel::threads::get_process_local;

/// Current process type (provides an object handle for IPC)
pub struct CurProcess;
impl ::objects::Object for CurProcess
{
	const CLASS: u16 = values::CLASS_CORE_THISPROCESS;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &Any { self }
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64, Error>
	{
		match call
		{
		values::CORE_THISPROCESS_RECVOBJ => {
			let class = try!( <u16>::get_arg(&mut args) );
			Ok( ::objects::get_unclaimed(class) )
			},
		_ => todo!("CurProcess::handle_syscall({}, ...)", call),
		}
	}
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32
	{
		let mut ret = 0;
		if flags & values::EV_THISPROCESS_RECVOBJ != 0 {
			::objects::wait_for_obj(obj);
			ret += 1;
		}
		ret
	}
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32
	{
		let mut ret = 0;
		if flags & values::EV_THISPROCESS_RECVOBJ != 0 {
			::objects::clear_wait_for_obj(obj);
			ret |= values::EV_THISPROCESS_RECVOBJ;
		}
		ret
	}
}

#[inline(never)]
pub fn exit(status: u32) {
	::kernel::threads::exit_process(status);
}
#[inline(never)]
pub fn terminate() {
	todo!("terminate()");
}
#[inline(never)]
pub fn newthread(sp: usize, ip: usize) -> ObjectHandle {
	todo!("newthread(sp={:#x},ip={:#x})", sp, ip);
}
#[inline(never)]
pub fn newprocess(name: &str, ip: usize, sp: usize, clone_start: usize, clone_end: usize) -> ObjectHandle {
	// 1. Create a new process image (virtual address space)
	let mut process = ::kernel::threads::ProcessHandle::new(name, clone_start, clone_end);
	
	// 3. Create a new thread using that process image with the specified ip/sp
	process.start_root_thread(ip, sp);
	
	struct Process(::kernel::threads::ProcessHandle);
	impl ::objects::Object for Process
	{
		const CLASS: u16 = values::CLASS_CORE_PROCESS;
		fn class(&self) -> u16 { Self::CLASS }
		fn as_any(&self) -> &Any { self }
		fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error>
		{
			match call
			{
			// Request termination of child process
			values::CORE_PROCESS_KILL => todo!("CORE_PROCESS_KILL"),
			// Send one of this process' objects to the child process
			values::CORE_PROCESS_SENDOBJ => {
				let handle = try!(<u32>::get_arg(&mut args));
				::objects::give_object(&self.0, handle).map(|_| 0)
				},
			_ => todo!("Process::handle_syscall({}, ...)", call),
			}
		}
		fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
			let mut ret = 0;
			// Wait for child process to terminate
			if flags & values::EV_PROCESS_TERMINATED != 0 {
				self.0.bind_wait_terminate(obj);
				ret += 1;
			}
			ret
		}
		fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
			let mut ret = 0;
			// Wait for child process to terminate
			if flags & values::EV_PROCESS_TERMINATED != 0 {
				if self.0.clear_wait_terminate(obj) {
					ret |= values::EV_PROCESS_TERMINATED;
				}
			}
			ret
		}
	}

	::objects::new_object( Process(process) )
}

// ret: number of events triggered
#[inline(never)]
pub fn wait(events: &mut [values::WaitItem], wake_time_mono: u64) -> Result<u32,Error>
{
	let mut waiter = ::kernel::threads::SleepObject::new("wait");
	let mut num_bound = 0;
	for ev in events.iter() {
		num_bound += try!(::objects::wait_on_object(ev.object, ev.flags, &mut waiter));
	}

	if num_bound == 0 && wake_time_mono == !0 {
		// Attempting to sleep on no events with an infinite timeout! Would sleep forever
		log_error!("TODO: What to do when a thread tries to sleep forever");
		waiter.wait();
	}

	// A wake time of 0 means to not sleep at all, just check the status of the events
	// TODO: There should be a more efficient way of doing this, than binding only to unbind again
	if wake_time_mono != 0 {
		// !0 indicates an unbounded wait (no need to set a wakeup time)
		if wake_time_mono != !0 {
			todo!("Set a wakeup timer at {}", wake_time_mono);
			//waiter.wait_until(wake_time_mono);
		}
		else {
			waiter.wait();
		}
	}

	Ok( events.iter_mut().fold(0, |total,ev| total + ::objects::clear_wait(ev.object, ev.flags, &mut waiter).unwrap()) )
}
