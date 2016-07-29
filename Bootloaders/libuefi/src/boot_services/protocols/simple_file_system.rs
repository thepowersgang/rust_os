
use {Status};

#[repr(C)]
pub struct SimpleFileSystem
{
	revision: u64,
	open_volume: extern "win64" fn(&SimpleFileSystem, &mut *mut super::File) -> Status,
}

impl super::Protocol for SimpleFileSystem
{
	fn guid() -> ::Guid {
		::Guid( 0x0964e5b22,0x6459,0x11d2, [0x8e,0x39,0x00,0xa0,0xc9,0x69,0x72,0x3b] )
	}
	unsafe fn from_ptr(v: *const ::Void) -> *const Self {
		v as *const _
	}
}

impl SimpleFileSystem
{
	pub fn open_volume(&self) -> Result< super::Owned<super::File>, Status > {
		let mut ptr = ::core::ptr::null_mut();
		(self.open_volume)(self, &mut ptr)
			// SAFE: Pointer passed to us for ownership
			.err_or_else(|| unsafe {super::Owned::from_ptr(ptr) } )
	}
}

