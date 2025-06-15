// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/args.rs
//! Argument de-marshalling
use kernel::memory::freeze::{Freeze,FreezeMut};

pub trait SyscallArg: Sized {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error>;
}

pub struct Args<'a>(&'a [usize]);
impl<'a> Args<'a>
{
	pub fn new(v: &'a [usize]) -> Self {
		Args(v)
	}
	pub fn get<T: SyscallArg>(&mut self) -> Result<T, crate::Error> {
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
unsafe impl Pod for crate::values::WaitItem {}
unsafe impl Pod for crate::values::GuiEvent {}	// Kinda lies, but meh
unsafe impl Pod for crate::values::RpcMessage {}


#[cfg(feature="native")]
extern "Rust" {
	fn native_map_syscall_pointer(ptr: *const u8, len: usize, is_mut: bool) -> *const u8;
}

impl<T: Pod> SyscallArg for Freeze<T>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let ptr = args[0] as *const T;
		let blen = ::core::mem::size_of::<T>();
		*args = &args[1..];
		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe {
			// 1. Check if the pointer is into user memory
			// TODO: ^^^
			// 2. Ensure that the pointed slice is valid (overlaps checks by Freeze, but gives a better error)
			// TODO: Replace this check with mapping FreezeError
			#[cfg(feature="native")]
			let ptr_real = native_map_syscall_pointer(ptr as *const u8, blen, false) as *const T;
			#[cfg(not(feature="native"))]
			let ptr_real = ptr;
			let bs = if let Some(v) = ::kernel::memory::buf_to_slice(ptr_real, 1) {
					&v[0]
				} else {
					return Err( crate::Error::InvalidBuffer(ptr as *const (), blen) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( Freeze::new(bs)? )
		}
	}
}
impl<T: Pod> SyscallArg for Freeze<[T]>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 2 {
			return Err( crate::Error::TooManyArgs );
		}
		let ptr = args[0] as *const T;
		let len = args[1];
		let blen = len * ::core::mem::size_of::<T>();
		*args = &args[2..];
		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe {
			// 1. Check if the pointer is into user memory
			// TODO: ^^^
			// 2. Ensure that the pointed slice is valid (overlaps checks by Freeze, but gives a better error)
			// TODO: Replace this check with mapping FreezeError
			#[cfg(feature="native")]
			let ptr_real = native_map_syscall_pointer(ptr as *const u8, blen, false) as *const T;
			#[cfg(not(feature="native"))]
			let ptr_real = ptr;
			let bs = if let Some(v) = ::kernel::memory::buf_to_slice(ptr_real, len) {
					v
				} else {
					return Err( crate::Error::InvalidBuffer(ptr as *const (), blen) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( Freeze::new(bs)? )
		}
	}
}
impl SyscallArg for Freeze<str> {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		let ret = Freeze::<[u8]>::get_arg(args)?;
		// SAFE: Transmuting [u8] to str is valid if the str is valid UTF-8
		unsafe { 
			::core::str::from_utf8(&ret)?;
			Ok(::core::mem::transmute(ret))
		}
	}
}
impl<T: Pod> SyscallArg for FreezeMut<T>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let ptr = args[0] as *mut T;
		let blen = ::core::mem::size_of::<T>();

		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe { 
			#[cfg(feature="native")]
			let ptr_real = native_map_syscall_pointer(ptr as *const u8, blen, /*is_mut*/true) as *mut T;
			#[cfg(not(feature="native"))]
			let ptr_real = ptr;
			let bs = if let Some(v) = ::kernel::memory::buf_to_slice_mut(ptr_real, 1) {
					v
				} else {
					return Err( crate::Error::InvalidBuffer(ptr as *const (), blen) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( FreezeMut::new(&mut bs[0])? )
		}
	}
}
impl<T: Pod> SyscallArg for FreezeMut<[T]>
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 2 {
			return Err( crate::Error::TooManyArgs );
		}
		let ptr = args[0] as *mut T;
		let len = args[1];
		let blen = len * ::core::mem::size_of::<T>();
		*args = &args[2..];
		// SAFE: Performs data validation, and only accepts user pointers (which are checkable)
		unsafe { 
			// 1. Check if the pointer is into user memory
			// TODO: ^^^
			// 2. Ensure that the pointed slice is valid (overlaps checks by Freeze, but gives a better error)
			// TODO: Replace this check with mapping FreezeError
			#[cfg(feature="native")]
			let ptr_real = native_map_syscall_pointer(ptr as *const u8, blen, /*is_mut*/true) as *mut T;
			#[cfg(not(feature="native"))]
			let ptr_real = ptr;
			let bs = if let Some(v) = ::kernel::memory::buf_to_slice_mut(ptr_real, len) {
					v
				} else {
					return Err( crate::Error::InvalidBuffer(ptr as *const (), blen) );
				};
			// 3. Create a freeze on that memory (ensuring that it's not unmapped until the Freeze object drops)
			Ok( FreezeMut::new(bs)? )
		}
	}
}

impl SyscallArg for crate::values::FixedStr8
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		let count = 8 / ::core::mem::size_of::<usize>();
		if args.len() < count {
			return Err( crate::Error::TooManyArgs );
		}
		let mut rv_bytes = [0; 8];
		if count == 2 {
			let v1 = args[0];
			let v2 = args[1];
			rv_bytes[..4].copy_from_slice( ::kernel::lib::as_byte_slice(&v1) );
			rv_bytes[4..].copy_from_slice( ::kernel::lib::as_byte_slice(&v2) );
			*args = &args[2..];
		}
		else {
			let v = args[0];
			rv_bytes.copy_from_slice( ::kernel::lib::as_byte_slice(&v) );
			*args = &args[1..];
		}
		Ok( crate::values::FixedStr8::from(rv_bytes) )
	}
}
impl SyscallArg for crate::values::FixedStr6
{
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		let count = (6 + ::core::mem::size_of::<usize>() - 1) / ::core::mem::size_of::<usize>();
		if args.len() < count {
			return Err( crate::Error::TooManyArgs );
		}
		let mut rv_bytes = [0; 6];
		if count == 2 {
			let v1 = args[0];
			let v2 = args[1];
			rv_bytes[..4].copy_from_slice( ::kernel::lib::as_byte_slice(&v1) );
			rv_bytes[4..].copy_from_slice( &::kernel::lib::as_byte_slice(&v2)[..2] );
			*args = &args[2..];
		}
		else {
			let v = args[0];
			rv_bytes.copy_from_slice( &::kernel::lib::as_byte_slice(&v)[..6] );
			*args = &args[1..];
		}
		Ok( crate::values::FixedStr6::from(rv_bytes) )
	}
}

impl SyscallArg for usize {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let rv = args[0];
		*args = &args[1..];
		Ok( rv )
	}
}
#[cfg(target_pointer_width="64")]
impl SyscallArg for u64 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let rv = args[0] as u64;
		*args = &args[1..];
		Ok( rv )
	}
}
#[cfg(target_pointer_width="32")]
impl SyscallArg for u64 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 2 {
			return Err( crate::Error::TooManyArgs );
		}
		let rv = args[0] as u64 | (args[1] as u64) << 32;
		*args = &args[2..];
		Ok( rv )
	}
}

impl SyscallArg for u32 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let rv = args[0] as u32;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for u16 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let rv = args[0] as u16;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for u8 {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let rv = args[0] as u8;
		*args = &args[1..];
		Ok( rv )
	}
}
impl SyscallArg for bool {
	fn get_arg(args: &mut &[usize]) -> Result<Self, crate::Error> {
		if args.len() < 1 {
			return Err( crate::Error::TooManyArgs );
		}
		let rv = (args[0] as u8) != 0;
		*args = &args[1..];
		Ok( rv )
	}
}

/*

// EXPERIMENT: A pile of traits to get argument structs from the `args` interface
// ISSUES:
// - For borrows, data needs to go via `Freeze` and `FreezeMut`, which opens a whole can of worms:
//   - Lifetime trickery (solved by passing `tmp` to the top-level function, see far below)
//   - Overlapping impls, so `&T` can have custom logic

pub trait SingleArg<'a>: 'a + Sized {
	type SrcTy: SyscallArg;
	fn get(src: &'a mut Self::SrcTy) -> Self;
}
impl<'a, T: Pod> SingleArg<'a> for &'a [T] {
	type SrcTy = Freeze<[T]>;
	fn get(src: &'a mut Self::SrcTy) -> Self {
		src
	}
}
impl<'a, T: Pod> SingleArg<'a> for &'a mut [T] {
	type SrcTy = FreezeMut<[T]>;
	fn get(src: &'a mut Self::SrcTy) -> Self {
		src
	}
}
impl<'a, T: Pod> SingleArg<'a> for &'a T {
	type SrcTy = Freeze<T>;
	fn get(src: &'a mut Self::SrcTy) -> Self {
		src
	}
}
impl<'a, T: Pod> SingleArg<'a> for &'a mut T {
	type SrcTy = FreezeMut<T>;
	fn get(src: &'a mut Self::SrcTy) -> Self {
		src
	}
}
impl<'a> SingleArg<'a> for usize {
	type SrcTy = Self;
	fn get(src: &'a mut Self::SrcTy) -> Self {
		*src
	}
}
/*
default impl<'a, T: 'a> SingleArg<'a> for T
where
	T: SyscallArg + Copy
{
	type SrcTy = T;
	fn get(src: &'a mut T) -> T {
		*src
	}
}
*/

#[doc(hidden)]
pub trait ArgsTuple<'a>: 'a {
	type Source;
	fn get_src(args: &mut Args) -> Result<Self::Source,crate::Error>;
	fn get(src: &'a mut Self::Source) -> Self;
}
impl<'a> ArgsTuple<'a> for ()
where
{
	type Source = ();
	fn get_src(_: &mut Args) -> Result<Self::Source,crate::Error> {
		Ok( () )
	}
	fn get(_: &'a mut Self::Source) -> Self {
		()
	}
}
impl<'a, A1> ArgsTuple<'a> for (A1,)
where
	A1: 'a + SingleArg<'a>,
{
	type Source = (A1::SrcTy,);
	fn get_src(args: &mut Args) -> Result<Self::Source,crate::Error> {
		Ok( (args.get()?,) )
	}
	fn get(src: &'a mut Self::Source) -> Self {
		(A1::get(&mut src.0),)
	}
}
impl<'a, A1, A2> ArgsTuple<'a> for (A1,A2,)
where
	A1: 'a + SingleArg<'a>,
	A2: 'a + SingleArg<'a>,
{
	type Source = (A1::SrcTy,A2::SrcTy,);
	fn get_src(args: &mut Args) -> Result<Self::Source,crate::Error> {
		Ok( (args.get()?, args.get()?, ) )
	}
	fn get(src: &'a mut Self::Source) -> Self {
		( A1::get(&mut src.0), A2::get(&mut src.1), )
	}
}
pub fn with_args<'a, A: 'a + ::syscall_values::Args>(
	args: &mut Args,
	tmp: &'a mut Option< < <A as ::syscall_values::Args>::Tuple as ArgsTuple<'a> >::Source>,
	fcn: impl FnOnce(A)->u64
) -> Result<u64,super::Error>
where
	A::Tuple: ArgsTuple<'a>,
{
	//let mut src = A::Tuple::get_src(args)?;
	let src = tmp.get_or_insert(A::Tuple::get_src(args)?);
	let args = A::Tuple::get(src);
	let args = A::from_tuple(args);
	Ok(fcn(args))
}
// */