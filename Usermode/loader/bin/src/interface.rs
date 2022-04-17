// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// interface.rs
// - Exposed process spawning interface

// Import the interface crate
extern crate loader;

extern "C" {
	static init_path: [u8; 0];
	static init_path_end: [u8; 0];
	static mut arg_count: u32;
}

static S_BUFFER_LOCK: ::syscalls::sync::Mutex<()> = ::syscalls::sync::Mutex::new( () );

impl_from! {
	From<NullStringBuilderError>(_v) for loader::Error {
		loader::Error::BadArguments
	}
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// Spawn a new process using the provided binary and arguments
pub extern "C" fn new_process(executable_handle: ::syscalls::vfs::File, process_name: &[u8], args: &[&[u8]]) -> Result<::syscalls::threads::ProtoProcess,loader::Error>
{
	extern "C" {
		static limit_and_base: [u64; 2];
	}
	
	kernel_log!("new_process({:?}, ...)", ::std::ffi::OsStr::new(process_name));
	
	// Acquire the global buffer lock and start the new process
	let proto_proc = {
		// Lock loader until after 'start_process', allowing global memory to be used as buffer for binary and arguments
		// - After start_process, we can safely release and reuse the memory (becuase this space is cloned into the new process)
		let _lh = S_BUFFER_LOCK.lock();
		
		// Store binary and arguments in .data
		// SAFE: Locked
		unsafe {
			arg_count = (args.len() + 1) as u32;
		}
		// SAFE: Locked (so access is unique), and pointers are valid
		let buf = unsafe {
			let buf_end = init_path_end.as_ptr() as usize;
			let buf_start = init_path.as_ptr() as usize;
			let len = buf_end - buf_start;
			assert!(buf_end > buf_start, "Init path symbols out of order: init_path_end({:#x}) !> init_path({:#x})", buf_end, buf_start );
			
			::std::slice::from_raw_parts_mut(buf_start as *mut u8, len)
			};
		let mut builder = NullStringBuilder( buf );
		builder.push( process_name )?;
		for arg in args {
			builder.push(arg)?;
		}
		
		let name = ::std::str::from_utf8(process_name).unwrap_or("BADSTR");

		// Spawn new process
		// SAFE: Just takes the address of the externs statics
		match ::syscalls::threads::start_process(name, unsafe { limit_and_base[0] as usize }, unsafe { limit_and_base[1] as usize })
		{
		Ok(v) => v,
		Err(e) => panic!("TODO: new_process - Error '{:?}'", e),
		}
		// - Lock is dropped here (for this process)
		};
	
	// Send the executable handle
	kernel_log!("- Sending root and executable handle");
	proto_proc.send_obj( "ro:/", ::syscalls::vfs::root().clone() );	// Must be first object created (name is not actually used)
	proto_proc.send_obj( "exec", executable_handle );

	kernel_log!("- Returning ProtoProcess");
	Ok(proto_proc)
}
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn start_process(pp: ::syscalls::threads::ProtoProcess) -> ::syscalls::threads::Process {
	extern "C" {
		static init_stack_end: [u8; 0];
	}
	// SAFE: Just takes the address
	pp.start( new_process_entry as usize, unsafe { init_stack_end.as_ptr() as usize } )
}

/// Entrypoint for new processes, runs with a clean stack
fn new_process_entry() -> !
{
	kernel_log!("new_process_entry");
	// Release buffer lock once in new process
	// SAFE: Unlocking our copy of the lock
	unsafe {
		S_BUFFER_LOCK.unlock();
	}
	// SAFE: Valid memory from linker script
	let (arg_slice, argc) = unsafe {
		assert!(arg_count > 0);
		let s = ::std::slice::from_raw_parts( init_path.as_ptr(), init_path_end.as_ptr() as usize - init_path.as_ptr() as usize );
		(s, arg_count)
		};
	
	// Parse command line stored in data area (including image path)
	let mut arg_iter = NullStringList(arg_slice).map(::std::ffi::OsStr::new);
	let process_name = arg_iter.next().expect("No binary was passed");
	kernel_log!("Binary = {:?}", process_name);
	let arg_iter = (0 .. argc).zip(arg_iter);
	for (i,arg) in arg_iter {
		kernel_log!("Arg {}: {:?}", i, arg);
	}

	let arg_iter = NullStringList(arg_slice).map(::std::ffi::OsStr::new).skip(1);
	let arg_iter = (0 .. argc).zip(arg_iter);
	
	
	
	let fh: ::syscalls::vfs::File = ::syscalls::threads::S_THIS_PROCESS.receive_object("exec").expect("Could not receive the executable vfs::File object");
	::syscalls::vfs::root();	// Fetches the root handle too
	let entrypoint = ::load_binary(process_name, fh);
	
	// TODO: Coordinate with the parent process and receive an initial set of objects (e.g. WM root)?
	// - Could possibly leave this up to user code, or at least std
	
	// Populate arguments
	let mut args = super::FixedVec::new();
	for (_,arg) in arg_iter {
		args.push(arg).unwrap();
	}
	kernel_log!("args = {:?}", &*args);
	
	// TODO: Switch stacks into a larger dynamically-allocated stack
	// SAFE: Entrypoint assumed to have this signature
	let ep: fn(&[&::std::ffi::OsStr]) -> ! = unsafe { ::std::mem::transmute(entrypoint) };
	kernel_log!("Calling entry {:p} for {:?}", ep as *const (), process_name);
	ep(&args);
}


#[derive(Clone)]
struct NullStringList<'a>(&'a [u8]);
impl<'a> Iterator for NullStringList<'a>
{
	type Item = &'a [u8];
	fn next(&mut self) -> Option<&'a [u8]> {
		if self.0.len() == 0 {
			None
		}
		else {
			if let Some(nul_pos) = self.0.iter().enumerate().find(|&(_,&v)| v == 0).map(|(i,_)| i)
			{
				let ret = &self.0[..nul_pos];
				self.0 = &self.0[nul_pos+1..];
				Some(ret)
			}
			else {
				let ret = &self.0[..];
				self.0 = &self.0[self.0.len()..];
				Some(ret)
			}
		}
	}
}

#[doc(hidden)]
pub enum NullStringBuilderError {
	ContainsNull,
	InsufficientSpace,
}
struct NullStringBuilder<'a>(&'a mut [u8]);
impl<'a> NullStringBuilder<'a>
{
	fn push(&mut self, bytes: &[u8]) -> Result<(), NullStringBuilderError> {
		if bytes.contains(&0) {
			Err( NullStringBuilderError::ContainsNull )
		}
		else if bytes.len() > self.0.len() {
			Err( NullStringBuilderError::InsufficientSpace )
		}
		else {
			let rem: *mut [u8] = if bytes.len() == self.0.len() {
					let (dst, rem) = self.0.split_at_mut(bytes.len());
					for (d,s) in dst.iter_mut().zip( bytes.iter() ) {
						*d = *s;
					}
					rem
				} else {
					let (dst, rem) = self.0.split_at_mut(bytes.len()+1);
					for (d,s) in dst.iter_mut().zip( bytes.iter() ) {
						*d = *s;
					}
					dst[bytes.len()] = b'\0';
					rem
				};
			// SAFE: (Fuck you borrowck)
			self.0 = unsafe { ::std::mem::transmute(rem) };
			Ok( () )
		}
	}
}

