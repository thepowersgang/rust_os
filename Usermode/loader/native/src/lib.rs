#![crate_type="cdylib"]

// TODO: Put this somewhere common, can't load `loader` here
#[derive(Debug)]
pub enum Error
{
	NotFound,
	NotExecutable,
	BadFormat,
	CorruptExecutable,
	BadArguments,
}

#[allow(improper_ctypes_definitions)]
#[no_mangle]
pub extern "C" fn new_process(executable_handle: ::syscalls::vfs::File, process_name: &[u8], args: &[&[u8]]) -> Result<::syscalls::threads::ProtoProcess,Error>
{
	// Send a special syscall that prepares the process
	// - Need to hand the executable handle to the server
	// 1. Pack the arguments into a NUL separated list
	let mut args_packed = Vec::new();
	for a in args {
		args_packed.extend(a.iter().copied());
		args_packed.push(0);
	}
	let name = ::std::str::from_utf8(process_name).unwrap_or("BADSTR");

	match ::syscalls::threads::start_process(executable_handle, name, &args_packed)
	{
	Ok(h) => Ok(h),
	Err(e) => todo!("loader native new_process: error={}", e),
	}
}

#[allow(improper_ctypes_definitions)]
#[no_mangle]
pub extern "C" fn start_process(handle: ::syscalls::threads::ProtoProcess) -> ::syscalls::threads::Process
{
	handle.start(0,0)
}

// TODO: use this for sending syscalls
#[no_mangle]
pub unsafe extern "C" fn rustos_naitive_syscall(id: u32, opts: &[usize]) -> u64 {
	use ::syscalls::values::*;
	match id
	{
	// Process-related core functions
	CORE_LOGWRITE => {
		let ptr = ::core::slice::from_raw_parts(opts[0] as *const u8, opts[1]);
		println!("LOGWRITE: {}", String::from_utf8_lossy(ptr));
		0
		},
	CORE_DBGVALUE => {
		let ptr = ::core::slice::from_raw_parts(opts[0] as *const u8, opts[1]);
		let val = opts[2];
		println!("LOGWRITE: {}: {:#x}", String::from_utf8_lossy(ptr), val);
		0
		},
	CORE_EXITPROCESS => {
		::std::process::exit(opts[0] as i32)
		},
	// Memory, wrap mmap
	MEM_ALLOCATE => todo!("MEM_ALLOCATE"),
	MEM_REPROTECT => todo!("MEM_REPROTECT"),
	MEM_DEALLOCATE => todo!("MEM_DEALLOCATE"),
	_ => {
		0
		},
	}
}

