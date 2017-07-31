//
//
/// Various object protocols

pub use self::loaded_image::LoadedImage;
pub use self::loaded_image_device_path::LoadedImageDevicePath;
pub use self::device_path::DevicePath;
pub use self::simple_file_system::SimpleFileSystem;
pub use self::graphics_output::GraphicsOutput;

pub use self::file::File;

pub trait Protocol
{
	fn guid() -> ::Guid;
	unsafe fn from_ptr(*const ::Void) -> *const Self;
}
pub trait OwnedProtocol
{
	#[doc(hidden)]
	unsafe fn drop(&mut self);
}

pub struct Owned<T: OwnedProtocol>(::core::ptr::Unique<T>);
impl<T> Owned<T>
where
	T: OwnedProtocol
{
	/// UNSAFE: Pointer must be valid to hand to this for ownership
	unsafe fn from_ptr(p: *mut T) -> Self {
		Owned( ::core::ptr::Unique::new_unchecked(p) )
	}
}
impl<T> ::core::ops::Drop for Owned<T>
where
	T: OwnedProtocol
{
	fn drop(&mut self) {
		// SAFE: Owned pointer
		unsafe {
			self.0.as_mut().drop();
		}
	}
}
impl<T> ::core::ops::Deref for Owned<T>
where
	T: OwnedProtocol
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Owned pointer
		unsafe { self.0.as_ref() }
	}
}
impl<T> ::core::ops::DerefMut for Owned<T>
where
	T: OwnedProtocol
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: Owned pointer
		unsafe { self.0.as_mut() }
	}
}


mod loaded_image;
mod loaded_image_device_path;
mod device_path;
mod simple_file_system;

mod graphics_output;
mod file;

