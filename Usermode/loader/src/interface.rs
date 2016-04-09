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
/// Spawn a new process using the provided binary and arguments
pub extern "C" fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<::syscalls::threads::ProtoProcess,loader::Error>
{
	extern "C" {
		static BASE: [u8; 0];
		static LIMIT: [u8; 0];
	}
	
	kernel_log!("new_process({:?}, ...)", ::std::ffi::OsStr::new(binary));

	let executable_handle = match ::syscalls::vfs::ROOT.open_child_path(binary)
		{
		Err(_) => return Err(loader::Error::NotFound),
		Ok(v) => match v.into_file(::syscalls::vfs::FileOpenMode::Execute)
			{
			Err(_) => return Err(loader::Error::NotExecutable),
			Ok(v) => v,
			}
		};
	
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
		builder.push( binary );
		for arg in args {
			try!( builder.push(arg) );
		}
		
		let name = ::std::str::from_utf8(binary).unwrap_or("BADSTR");

		// Spawn new process
		match ::syscalls::threads::start_process(name, BASE.as_ptr() as usize, LIMIT.as_ptr() as usize)
		{
		Ok(v) => v,
		Err(e) => panic!("TODO: new_process - Error '{:?}'", e),
		}
		// - Lock is dropped here (for this process)
		};
	
	// Send the executable handle
	kernel_log!("- Sending executable handle");
	proto_proc.send_obj( executable_handle );	

	kernel_log!("- Returning ProtoProcess");
	Ok(proto_proc)
}
#[no_mangle]
pub extern "C" fn start_process(pp: ::syscalls::threads::ProtoProcess) -> ::syscalls::threads::Process {
	extern "C" {
		static init_stack_end: [u8; 0];
	}
	pp.start( new_process_entry as usize, init_stack_end.as_ptr() as usize )
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
	let binary = arg_iter.next().expect("No binary was passed");
	let arg_iter = (0 .. argc).zip(arg_iter);
	kernel_log!("Binary = {:?}", binary);
	for (i,arg) in arg_iter {
		kernel_log!("Arg {}: {:?}", i, arg);
	}

	let arg_iter = NullStringList(arg_slice).map(::std::ffi::OsStr::new).skip(1);
	let arg_iter = (0 .. argc).zip(arg_iter);
	
	
	
	let fh: ::syscalls::vfs::File = ::syscalls::threads::S_THIS_PROCESS.receive_object().expect("Could not receive the executable vfs::File object");
	let entrypoint = ::load_binary(binary, fh);
	
	// TODO: Coordinate with the parent process and receive an initial set of objects (e.g. WM root)?
	// - Could possibly leave this up to user code, or at least std
	
	// Populate arguments
	let mut args = super::FixedVec::new();
	//args.push(binary).unwrap();
	for (_,arg) in arg_iter {
		args.push(arg).unwrap();
	}
	kernel_log!("args = {:?}", &*args);
	
	// TODO: Switch stacks into a larger dynamically-allocated stack
	// SAFE: Entrypoint assumed to have this signature
	let ep: fn(&[&::std::ffi::OsStr]) -> ! = unsafe { ::std::mem::transmute(entrypoint) };
	kernel_log!("Calling entry {:p} for {:?}", ep as *const (), binary);
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

