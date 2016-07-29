
use {Status};

const FILE_MODE_READ: u64 = 1;
const FILE_MODE_WRITE: u64 = 2;
const FILE_MODE_CREATE: u64 = 1 << 63;

#[repr(C)]
pub struct File
{
	revision: u64,
	open: extern "win64" fn(&File, &mut *mut File, *const u16, u64, u64) -> Status,
	close: extern "win64" fn(&mut File)->Status,
	delete: extern "win64" fn()->Status,
	read: extern "win64" fn()->Status,
	write: extern "win64" fn()->Status,
	get_position: extern "win64" fn()->Status,
	set_position: extern "win64" fn()->Status,
}

impl File
{
	pub fn open_read(&self, path: &[u16]) -> Result< super::Owned<File>, Status > {
		let mut out = ::core::ptr::null_mut();
		(self.open)(self, &mut out, path.as_ptr(), FILE_MODE_READ, 0)
			// SAFE: Pointer has been passed to us for ownership
			.err_or_else(|| unsafe { super::Owned::from_ptr(out) } )
	}
}

impl super::OwnedProtocol for File
{
	unsafe fn drop(&mut self) {
		(self.close)(self);
	}
}

