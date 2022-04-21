//
//
//
//use prelude::*;

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
type _Unwind_Action = u32;
static _UA_SEARCH_PHASE: _Unwind_Action = 1;

#[allow(non_camel_case_types)]
#[repr(C)]
struct _Unwind_Exception
{
	exception_class: u64,
	exception_cleanup: fn(_Unwind_Reason_Code,*const _Unwind_Exception),
	private: [u64; 2],
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
#[panic_handler]
pub extern fn rust_begin_unwind(info: &::core::panic::PanicInfo) -> ! {
	let file_line = match info.location()
		{
		Some(v) => (v.file(), v.line()),
		None => ("", 0),
		};
	if let Some(m) = info.payload().downcast_ref::<::core::fmt::Arguments>() {
		begin_panic_fmt(m, file_line)
	}
	else if let Some(m) = info.payload().downcast_ref::<&str>() {
		begin_panic_fmt(&format_args!("{}", m), file_line)
	}
	else if let Some(m) = info.message() {
		begin_panic_fmt(m, file_line)
	}
	else {
		begin_panic_fmt(&format_args!("Unknown"), file_line)
	}
}
fn begin_panic_fmt(msg: &::core::fmt::Arguments, (file, line): (&str, u32)) -> !
{
	static NESTED: ::core::sync::atomic::AtomicBool = ::core::sync::atomic::AtomicBool::new(false);
	crate::arch::puts("\nERROR: rust_begin_unwind: ");
	crate::arch::puts(file);
	crate::arch::puts(":");
	crate::arch::puth(line as u64);
	crate::arch::puts("\n");
	if NESTED.swap(true, ::core::sync::atomic::Ordering::SeqCst) {
		crate::arch::puts("NESTED!\n");
		loop {}
	}
	crate::arch::print_backtrace();
	log_panic!("{}:{}: Panicked \"{:?}\"", file, line, msg);
	crate::metadevs::video::set_panic(file, line as usize, msg);
	loop{}
}
#[lang="eh_personality"]
fn rust_eh_personality(
	version: isize, _actions: _Unwind_Action, _exception_class: u64,
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
	crate::arch::puts("\nABORT ABORT ABORT\n");
	crate::arch::print_backtrace();
	loop {}
}

// vim: ft=rust
