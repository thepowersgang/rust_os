
#[repr(C)]
#[derive(Debug)]
pub struct LoadedImageDevicePath(super::DevicePath);

impl super::Protocol for LoadedImageDevicePath
{
	fn guid() -> ::Guid {
		::Guid( 0xbc62157e,0x3e33,0x4fec, [0x99,0x20,0x2d,0x3b,0x36,0xd7,0x50,0xdf] )
	}
	unsafe fn from_ptr(ptr: *const ::Void) -> *const Self {
		ptr as *const _
	}
}

