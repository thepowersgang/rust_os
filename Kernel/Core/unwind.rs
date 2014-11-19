//
//
//
//use _common::*;

use self::_Unwind_Reason_Code::*;

#[allow(non_camel_case_types)]
#[repr(C)]
enum _Unwind_Reason_Code
{
	_URC_NO_REASON = 0,
	_URC_FOREIGN_EXCEPTION_CAUGHT = 1,
	_URC_FATAL_PHASE2_ERROR = 2,
	_URC_FATAL_PHASE1_ERROR = 3,
	_URC_NORMAL_STOP = 4,
	_URC_END_OF_STACK = 5,
	_URC_HANDLER_FOUND = 6,
	_URC_INSTALL_CONTEXT = 7,
	_URC_CONTINUE_UNWIND = 8,
}

#[allow(non_camel_case_types)]
struct _Unwind_Context;

#[allow(non_camel_case_types)]
type _Unwind_Action = uint;
static _UA_SEARCH_PHASE: uint = 1;

#[allow(non_camel_case_types)]
#[repr(C)]
struct _Unwind_Exception
{
	exception_class: u64,
	exception_cleanup: fn(_Unwind_Reason_Code,*const _Unwind_Exception),
	private: [u64, ..2],
}

/*
#[repr(C)]
struct Exception
{
	header: _Unwind_Exception,
	cause: (),
}

extern "C" {
	fn _Unwind_RaiseException(ex: *const _Unwind_Exception) -> !;
}

static EXCEPTION_CLASS : u64 = 0x544B3120_52757374;	// TK1 Rust (big endian)
// */

// Evil fail when doing unwind
#[no_mangle] 
pub extern "C" fn rust_begin_unwind(msg: &::core::fmt::Arguments, file: &'static str, line: uint) -> !
{
	::arch::puts("\nERROR: rust_begin_unwind\n");
	::arch::print_backtrace();
	log_panic!("rust_begin_unwind(msg=\"{}\", file=\"{}\", line={})", msg, file, line);
	/*
	unsafe {
		let ex = box Exception {
			cause: (),
			header: _Unwind_Exception {
				exception_class: EXCEPTION_CLASS,
				exception_cleanup: cleanup,
				private: [0,0],
				}
			};
		let exptr: *const _Unwind_Exception = ::core::mem::transmute(ex);
		log_trace!("exptr = {}", exptr);
		_Unwind_RaiseException( exptr );
	}
	
	fn cleanup(urc: _Unwind_Reason_Code, exception: *const _Unwind_Exception) {
	}
	// */
	loop{}
}
#[lang="eh_personality"]
fn rust_eh_personality(
	version: int, _actions: _Unwind_Action, _exception_class: u64,
	_exception_object: &_Unwind_Exception, _context: &_Unwind_Context
	) -> _Unwind_Reason_Code
{
	log_debug!("rust_eh_personality(version={},_actions={},_exception_class={:#x})",
		version, _actions, _exception_class);
	if version != 1 {
		log_error!("version({}) != 1", version);
		return _URC_FATAL_PHASE1_ERROR;
	}
	loop{}
}

#[no_mangle] pub extern "C" fn abort() -> !
{
	::arch::puts("\nABORT ABORT ABORT\n");
	::arch::print_backtrace();
	loop {}
}

// vim: ft=rust
