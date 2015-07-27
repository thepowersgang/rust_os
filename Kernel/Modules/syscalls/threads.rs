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

/// Current process type (provides an object handle for IPC)
pub struct CurProcess;
impl ::objects::Object for CurProcess {
	const CLASS: u16 = values::CLASS_CORE_THISPROCESS;
	fn class(&self) -> u16 { Self::CLASS }
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64, Error> {
		match call
		{
		values::CORE_THISPROCESS_RECVOBJ => {
			let class = try!( <u16>::get_arg(&mut args) );
			let idx = try!( <usize>::get_arg(&mut args) );
			Ok( ::objects::get_unclaimed(class, idx) )
			},
		values::CORE_THISPROCESS_RECVMSG => {
			let dest = try!( <FreezeMut<[u8]>>::get_arg(&mut args) );
			todo!("CORE_THISPROCESS_RECVMSG - {:p}+{}", dest.as_ptr(), dest.len());
			},
		_ => todo!("CurProcess::handle_syscall({}, ...)", call),
		}
	}
    fn bind_wait(&self, flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
        if flags & values::EV_THISPROCESS_RECVOBJ != 0 {
            todo!("EV_THISPROCESS_RECVOBJ");
        }
        if flags & values::EV_THISPROCESS_RECVMSG != 0 {
            todo!("EV_THISPROCESS_RECVOBJ");
        }
        0
    }
    fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
        unimplemented!();
    }
}

pub fn exit(status: u32) {
	todo!("exit(status={:x})", status);
}
pub fn terminate() {
	todo!("terminate()");
}
pub fn newthread(sp: usize, ip: usize) -> ObjectHandle {
	todo!("newthread(sp={:#x},ip={:#x})", sp, ip);
}
pub fn newprocess(name: &str, ip: usize, sp: usize, clone_start: usize, clone_end: usize) -> ObjectHandle {
	// 1. Create a new process image (virtual address space)
	let mut process = ::kernel::threads::ProcessHandle::new(name, clone_start, clone_end);
	
	// 3. Create a new thread using that process image with the specified ip/sp
	process.start_root_thread(ip, sp);
	
	struct Process(::kernel::threads::ProcessHandle);
	impl ::objects::Object for Process {
		const CLASS: u16 = values::CLASS_CORE_PROCESS;
		fn class(&self) -> u16 { Self::CLASS }
		fn handle_syscall(&self, call: u16, _args: &[usize]) -> Result<u64,Error> {
			match call
			{
			values::CORE_PROCESS_KILL => todo!("CORE_PROCESS_KILL"),
			values::CORE_PROCESS_SENDOBJ => todo!("CORE_PROCESS_SENDOBJ"),
			values::CORE_PROCESS_SENDMSG => todo!("CORE_PROCESS_SENDMSG"),
			_ => todo!("Process::handle_syscall({}, ...)", call),
			}
		}
		fn bind_wait(&self, flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 {
			if flags & values::EV_PROCESS_TERMINATED != 0 {
				todo!("EV_PROCESS_TERMINATED");
			}
			0
		}
		fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	}

	::objects::new_object( Process(process) )
}

// ret: number of events triggered
pub fn wait(events: &mut [values::WaitItem], wake_time_mono: u64) -> Result<u32,Error>
{
	let mut waiter = ::kernel::threads::SleepObject::new("wait");
	let mut num_bound = 0;
	for ev in events.iter() {
		num_bound += try!(::objects::wait_on_object(ev.object, ev.flags, &mut waiter));
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
			//waiter.wait_until(wake_time_mono);
		}
		else {
			waiter.wait();
		}
	}

	Ok( events.iter_mut().fold(0, |total,ev| total + ::objects::clear_wait(ev.object, ev.flags, &mut waiter).unwrap()) )
}
