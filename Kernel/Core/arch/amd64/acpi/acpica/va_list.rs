// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/acpica/va_list.rs
//! C FFI - va_list support
//!
//! NOTE: Current version is amd64 specific

#[repr(C)]
#[derive(Debug)]
#[allow(raw_pointer_derive)]
struct VaListInner
{
	gp_offset: u32,
	fp_offset: u32,
	overflow_arg_area: *const (),
	reg_save_area: *const (),
}
#[allow(non_camel_case_types)]
pub struct va_list(*mut VaListInner);
impl ::core::marker::Copy for va_list {}
impl ::core::clone::Clone for va_list { fn clone(&self) -> Self { *self } }

trait VaPrimitive
{
	unsafe fn get(&mut VaListInner) -> Self;
}

impl va_list
{
	pub unsafe fn get<T: VaPrimitive>(&mut self) -> T {
		//log_debug!("inner = {:p} {:?}", self.0, *self.0);
		T::get(&mut *self.0)
	}
}

impl VaListInner
{
	fn check_space(&self, num_gp: u32, num_fp: u32) -> bool {
		!(self.gp_offset > 48 - num_gp * 8 || self.fp_offset > 304 - num_fp * 16)
	}
	unsafe fn get_gp<T>(&mut self) -> T {
		let n_gp = (::core::mem::size_of::<T>()+7)/8;
		assert!( self.check_space(n_gp as u32, 0) );
		let rv = ::core::ptr::read( (self.reg_save_area as usize + self.gp_offset as usize) as *const _ );
		self.gp_offset += (8*n_gp) as u32;
		rv
	}
	unsafe fn get_overflow<T>(&mut self) -> T {
		let align = ::core::mem::min_align_of::<T>();
		// 7. Align overflow_reg_area upwards to a 16-byte boundary if alignment
		//    needed by T exceeds 8 bytes
		let addr = self.overflow_arg_area as usize;
		if align > 8 {
			if addr % 16 != 0 {
				self.overflow_arg_area = ((addr + 15) & !(16-1)) as *const _;
			}
		}
		else {
			if addr % 8 != 0 {
				self.overflow_arg_area = ((addr + 7) & !(8-1)) as *const _;
			}
		}
		// 8. Fetch from overflow areay
		let rv = ::core::ptr::read( self.overflow_arg_area as *const _ );
		self.overflow_arg_area = ((self.overflow_arg_area as usize) + ::core::mem::size_of::<T>()) as *const _;
		rv
	}
}

impl<T> VaPrimitive for *const T
{
	unsafe fn get(inner: &mut VaListInner) -> Self {
		<usize>::get(inner) as *const T
	}
}

macro_rules! impl_va_prim {
	($u:ty, $s:ty) => {
		impl VaPrimitive for $u {
			unsafe fn get(inner: &mut VaListInner) -> Self {
				// See the ELF AMD64 ABI document for a description of how this should act
				if ! inner.check_space(1, 0) {
					inner.get_overflow()
				}
				else {
					inner.get_gp()
				}
			}
		}
		impl VaPrimitive for $s {
			unsafe fn get(inner: &mut VaListInner) -> Self {
				::core::mem::transmute( <$u>::get(inner) )
			}
		}
	};
}

impl_va_prim!{ usize, isize }
impl_va_prim!{ u64, i64 }
impl_va_prim!{ u32, i32 }
impl_va_prim!{ u16, i16 }
impl_va_prim!{ u8, i8 }

