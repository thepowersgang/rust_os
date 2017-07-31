
#[repr(C)]
pub struct DevicePath
{
	ty: u8,
	sub_type: u8,
	length: [u8; 2],
}

impl DevicePath
{
	#[inline]
	pub fn type_code(&self) -> (u8,u8) {
		(self.ty, self.sub_type)
	}
	#[inline]
	fn data_ptr(&self) -> *const u8 {
		(self.length.as_ptr() as usize + 4) as *const u8
	}
	#[inline]
	fn data_len(&self) -> usize {
		self.length[0] as usize + self.length[1] as usize * 256
	}
	#[inline]
	fn data(&self) -> &[u8] {
		unsafe {
			::core::slice::from_raw_parts(self.data_ptr(), self.data_len())
		}
	}
}


impl super::Protocol for DevicePath
{
	fn guid() -> ::Guid {
		::Guid(0x09576e91,0x6d3f,0x11d2, [0x8e,0x39,0x00,0xa0,0xc9,0x69,0x72,0x3b])
	}
	unsafe fn from_ptr(ptr: *const ::Void) -> *const Self {
		ptr as *const DevicePath
	}
}
impl ::core::fmt::Debug for DevicePath
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match (self.ty, self.sub_type)
		{
		// ACPI Device Path (simple)
		(2, 1) => {
			#[repr(C)]
			struct AcpiDev {
				hid: u32,
				uid: u32,
			}
			let info = unsafe { &*(self.data_ptr() as *const AcpiDev) };
			write!(f, "ACPI:{:08x}/{:08x}", info.hid, info.uid)
			},
		// File path
		(4, 4) => {
			let s16 = unsafe { ::Str16::from_slice( ::core::slice::from_raw_parts( self.data_ptr() as *const u16, self.data_len() / 2 ) ) };
			write!(f, "\"{}\"", s16)
			},
		(_, _) => write!(f, "{{ty: {}, sub_type: {}, data: {:?}}}",
				self.ty, self.sub_type, self.data()
				),
		}
	}
}

