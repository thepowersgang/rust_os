// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/args.rs
//! Argument de-marshalling
use kernel::memory::freeze::{Freeze,FreezeMut};

pub trait SyscallArg: Sized {
	fn get_arg(args: &mut &[usize]) -> Result<Self,::Error>;
}

pub struct Args<'a>(&'a [usize]);
impl<'a> Args<'a>
{
	pub fn new(v: &[usize]) -> Args {
		Args(v)
	}
	pub fn get<T: SyscallArg>(&mut self) -> Result<T, ::Error> {
		T::get_arg(&mut self.0)
	}
}
impl<'a> ::core::fmt::Debug for Args<'a> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "[{:#x}]", ::kernel::lib::FmtSlice(self.0))
	}
}

// POD - Plain Old Data
pub unsafe trait Pod { }
unsafe impl Pod for u8 {}
unsafe impl Pod for u32 {}
unsafe impl Pod for ::values::WaitItem {}
unsafe impl Pod for ::values::GuiEvent {}	// Kinda lies, but meh

impl<T: Pod> SyscallArg for Freeze<[T]>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 2 {
			return Err( ::Error::TooManyArgs );
		}
		let ptr = args[0] as *const T;
		let len = args[1];
		*args = &args[2..];
		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe {
			// 1. Check if the pointer is into user memory
			// TODO: ^^^
			// 2. Ensure that the pointed slice is valid (overlaps checks by Freeze, but gives a better error)
			// TODO: Replace this check with mapping FreezeError
			let bs = if let Some(v) = ::kernel::memory::buf_to_slice(ptr, len) {
					v
				} else {
					return Err( ::Error::InvalidBuffer(ptr as *const (), len) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( try!(Freeze::new(bs)) )
		}
	}
}
impl SyscallArg for Freeze<str> {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		let ret = try!(Freeze::<[u8]>::get_arg(args));
		// SAFE: Transmuting [u8] to str is valid if the str is valid UTF-8
		unsafe { 
			try!( ::core::str::from_utf8(&ret) );
			Ok(::core::mem::transmute(ret))
		}
	}
}
impl<T: Pod> SyscallArg for FreezeMut<T>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 1 {
			return Err( ::Error::TooManyArgs );
		}
		let ptr = args[0] as *mut T;

		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe { 
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( try!(FreezeMut::new(&mut *ptr)) )
		}
	}
}
impl<T: Pod> SyscallArg for FreezeMut<[T]>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 2 {
			return Err( ::Error::TooManyArgs );
		}
		let ptr = args[0] as *mut T;
		let len = args[1];
		*args = &args[2..];
		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe { 
			// 1. Check if the pointer is into user memory
			// TODO: ^^^
			// 2. Ensure that the pointed slice is valid (overlaps checks by Freeze, but gives a better error)
			// TODO: Replace this check with mapping FreezeError
			let bs =  if let Some(v) = ::kernel::memory::buf_to_slice_mut(ptr, len) {	
					v
				} else {
					return Err( ::Error::InvalidBuffer(ptr as *const (), len) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( try!(FreezeMut::new(bs)) )
		}
	}
}

impl SyscallArg for usize {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 1 {
			return Err( ::Error::TooManyArgs );
		}
		let rv = args[0];
		*args = &args[1..];
		Ok( rv )
	}
}
#[cfg(target_pointer_width="64")]
impl SyscallArg for u64 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 1 {
			return Err( ::Error::TooManyArgs );
		}
		let rv = args[0] as u64;
		*args = &args[1..];
		Ok( rv )
	}
}
#[cfg(target_pointer_width="32")]
impl SyscallArg for u64 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 2 {
			return Err( ::Error::TooManyArgs );
		}
		let rv = args[0] as u64 | (args[1] as u64) << 32;
		*args = &args[2..];
		Ok( rv )
	}
}

impl SyscallArg for u32 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 1 {
			return Err( ::Error::TooManyArgs );
		}
		let rv = args[0] as u32;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for u16 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 1 {
			return Err( ::Error::TooManyArgs );
		}
		let rv = args[0] as u16;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for u8 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 1 {
			return Err( ::Error::TooManyArgs );
		}
		let rv = args[0] as u8;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for bool {
	fn get_arg(args: &mut &[usize]) -> Result<Self, ::Error> {
		if args.len() < 1 {
			return Err( ::Error::TooManyArgs );
		}
		let rv = (args[0] as u8) != 0;
		*args = &args[1..];
		Ok( rv )
	}
}
