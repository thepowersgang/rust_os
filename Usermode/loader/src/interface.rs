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
	static arg_count: u32;
}

//static S_BUFFER_LOCK: ::tifflin_syscalls::core::Futex = ::tifflin_syscalls::core::Futex::new();

impl_from! {
	From<NullStringBuilderError>(_v) for loader::Error {
		loader::Error::BadArguments
	}
}

#[no_mangle]
pub extern "C" fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<::tifflin_syscalls::threads::Process,loader::Error>
{
	extern "C" {
		static BASE: [u8; 0];
		static LIMIT: [u8; 0];
		static init_stack_end: [u8; 0];
		
		static mut arg_count: u32;
	}
	
	kernel_log!("new_process('{:?}', ...)", ::std::ffi::OsStr::new(binary));
	
	// Lock loader until after 'start_process', allowing global memory to be used as buffer for binary and arguments
	//let lh = S_BUFFER_LOCK.lock();
	
	// Store binary and arguments in .data
	unsafe
	{
		arg_count = (args.len() + 1) as u32;
		let len = init_path_end.as_ptr() as usize - init_path.as_ptr() as usize;
		let mut builder = NullStringBuilder( ::std::slice::from_raw_parts_mut(init_path.as_ptr() as *mut u8, len) );
		try!( builder.push(binary) );
		for arg in args {
			try!( builder.push(arg) );
		}
	}
	
	// Spawn new process
	match ::tifflin_syscalls::threads::start_process(new_process_entry as usize, init_stack_end.as_ptr() as usize, BASE.as_ptr() as usize, LIMIT.as_ptr() as usize)
	{
	Ok(v) => Ok( v ),
	Err(e) => panic!("TODO: new_process - Error '{:?}'", e),
	}
	
	// - Lock is dropped here (for this process)
}

/// Entrypoint for new processes, runs with a clean stack
fn new_process_entry() -> !
{
	kernel_log!("new_process_entry");
	// Release buffer lock once in new process
	//S_BUFFER_LOCK.release();
	assert!(arg_count > 0);
	let arg_slice = unsafe { ::std::slice::from_raw_parts( init_path.as_ptr(), init_path_end.as_ptr() as usize - init_path.as_ptr() as usize ) };
	
	// Parse command line stored in data area (including image path)
	let mut arg_iter = NullStringList(arg_slice).map(::std::ffi::OsStr::new);
	let binary = arg_iter.next().expect("No binary was passed");
	let arg_iter = (0 .. arg_count-1).zip(arg_iter);
	kernel_log!("Binary = {:?}", binary);
	for (i,arg) in arg_iter {
		kernel_log!("Arg {}: {:?}", i, arg);
	}

	let arg_iter = NullStringList(arg_slice).map(::std::ffi::OsStr::new);
	let arg_iter = (0 .. arg_count-1).zip(arg_iter);
	
	
	
	
	let entrypoint = ::load_binary(binary);
	
	// Populate arguments
	// TODO: Replace this mess with a FixedVec of some form
	// SAFE: We will be writing to this before reading from it
	let mut args = super::FixedVec::new();
	//args.push(binary).unwrap();
	for (_,arg) in arg_iter {
		args.push(arg).unwrap();
	}
	kernel_log!("args = {:?}", &*args);
	
	// TODO: Switch stacks into a larger dynamically-allocated stack
	let ep: fn(&[&::std::ffi::OsStr]) -> ! = unsafe { ::std::mem::transmute(entrypoint) };
	kernel_log!("Calling entry {:p}", ep as *const ());
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
			if let Some(nul_pos) = self.0.position_elem(&0)
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

