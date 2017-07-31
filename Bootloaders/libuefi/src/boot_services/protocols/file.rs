
use {Status};

const FILE_MODE_READ: u64 = 1;
const FILE_MODE_WRITE: u64 = 2;
const FILE_MODE_CREATE: u64 = 1 << 63;

#[repr(C)]
pub struct File
{
	revision: u64,
	open: efi_fcn!{ fn(&File, &mut *mut File, *const u16, u64, u64) -> Status },
	close: efi_fcn!{ fn(&mut File) -> Status },
	delete: efi_fcn!{ fn(&mut File) -> Status },
	read : efi_fcn!{ fn(&mut File, &mut usize, *mut ::Void) -> Status },
	write: efi_fcn!{ fn(&mut File, &mut usize, *const ::Void) -> Status },
	get_position: efi_fcn!{ fn(&File, &mut u64) -> Status },
	set_position: efi_fcn!{ fn(&mut File, u64) -> Status },
}

impl File
{
	#[inline]
	pub fn open_read(&self, path: &[u16]) -> Result< super::Owned<File>, Status > {
		let mut out = ::core::ptr::null_mut();
		assert!( path.last() == Some(&0) );
		// SAFE: File path is NUL-terminated
		(unsafe { (self.open)(self, &mut out, path.as_ptr(), FILE_MODE_READ, 0) })
			// SAFE: Pointer has been passed to us for ownership
			.err_or_else(|| unsafe { super::Owned::from_ptr(out) } )
	}

	#[inline]
	pub fn read(&mut self, data: &mut [u8]) -> Result<usize, Status> {
		let mut count = data.len();
		// SAFE: Data is valid up to passed length
		unsafe {
			(self.read)(self, &mut count, data.as_mut_ptr() as *mut _)
				.err_or_else(|| count)
		}
	}

	#[inline]
	pub fn get_position(&self) -> Result<u64, Status> {
		let mut pos = 0;
		// SAFE: Valid pointers
		(unsafe { (self.get_position)(self, &mut pos) })
			.err_or_else(|| pos)
	}
	#[inline]
	pub fn set_position(&mut self, pos: u64) -> Result<(), Status> {
		// SAFE: Valid pointers
		unsafe {
			(self.set_position)(self, pos).err_or( () )
		}
	}
}

impl super::OwnedProtocol for File
{
	#[inline]
	unsafe fn drop(&mut self) {
		(self.close)(self);
	}
}

