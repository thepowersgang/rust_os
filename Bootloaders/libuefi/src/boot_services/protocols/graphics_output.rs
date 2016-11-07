///
///
///
use {Status,Guid};

/// Protocol GUID
pub const GUID: Guid = Guid(0x9042a9de,0x23dc,0x4a38,[0x96,0xfb,0x7a,0xde,0xd0,0x80,0x51,0x6a]);

#[repr(C)]
pub struct GraphicsOutput
{
	query_mode: extern "win64" fn(&GraphicsOutput, u32, &mut usize, &mut *const ModeInformation)->Status,
	set_mode: extern "win64" fn(&GraphicsOutput, u32)->Status,
	blt: extern "win64" fn(&GraphicsOutput, *mut BltPixel, BltOperation, usize,usize, usize,usize, usize,usize,usize)->Status,
	mode: &'static Mode,
}
impl super::Protocol for GraphicsOutput
{
	fn guid() -> ::Guid {
		GUID
	}
	unsafe fn from_ptr(ptr: *const ::Void) -> *const Self {
		ptr as *const GraphicsOutput
	}
}

impl GraphicsOutput
{
	pub fn query_mode(&self, index: u32) -> Result<ModeInformation,Status> {
		let mut ptr = ::core::ptr::null();
		let mut size = 0;
		(self.query_mode)(self, index, &mut size, &mut ptr).err_or(())?;
		assert!(size >= ::core::mem::size_of::<ModeInformation>());
		// SAFE: (Assumed) The value is from the library, and should be valid to at least the size of ModeInformation
		let rv = unsafe { ::core::ptr::read(ptr) };
		// TODO: Should `ptr` be deallocated? The spec doesn't say anything except "callee-allocated buffer"
		Ok( rv )
	}
	pub fn set_mode(&self, index: u32) -> Result<(),Status> {
		(self.set_mode)(self, index).err_or(())
	}

	pub fn iter_modes(&self) -> ModeIter {
		ModeIter(self, 0)
	}
	
	pub fn blt_fill(&self, px: BltPixel, width: usize, height: usize,  dst_x: usize, dst_y: usize) {
		let _ = (self.blt)(self, &px as *const _ as *mut _, BltOperation::VideoFill, width, height, 0,0, dst_x,dst_y, 0);
	}
	pub fn blt_to_video(&self, data: &[BltPixel], width: usize, dst_x: usize, dst_y: usize) {
		let _ = (self.blt)(self, data.as_ptr() as *mut _, BltOperation::BufferToVideo, width, data.len() / width, 0,0, dst_x,dst_y, 0);
	}
	pub fn blt_from_video(&self, data: &mut [BltPixel], width: usize, src_x: usize, src_y: usize) {
		let _ = (self.blt)(self, data.as_mut_ptr(), BltOperation::VideoToBltBuffer, width, data.len() / width, src_x,src_y, 0,0, 0);
	}
	pub fn blt_inner_video(&self, src_x: usize, src_y: usize,  width: usize, height: usize,  dst_x: usize, dst_y: usize) {
		let _ = (self.blt)(self, ::core::ptr::null_mut(), BltOperation::VideoToVideo, width, height, src_x,src_y, dst_x,dst_y, 0);
	}
}

pub struct ModeIter<'a>(&'a GraphicsOutput, u32);
impl<'a> Iterator for ModeIter<'a>
{
	type Item = ModeInformation;
	fn next(&mut self) -> Option<ModeInformation> {
		if self.1 == self.0.mode.max_mode {
			None
		}
		else {
			self.1 += 1;
			self.0.query_mode(self.1).ok()
		}
	}
}

#[repr(C)]
pub struct ModeInformation
{
	version: u32,
	pub horizontal_resolution: u32,
	pub vertical_resolution: u32,
	pub pixel_format: PixelFormat,
	pub pixel_information: PixelBitmask,
	pub pixels_per_scanline: u32,
}

#[repr(C)]
pub struct PixelBitmask
{
	red_mask: u32,
	green_mask: u32,
	blue_mask: u32,
	reserved_mask: u32,
}

#[repr(C)]
pub enum PixelFormat
{
	RGBX,
	BGRX,
	BitMask,
	BltOnly,
}

#[repr(C)]
pub struct Mode
{
	max_mode: u32,
	mode: u32,
	info: *const ModeInformation,
	size_of_info: usize,
	frame_buffer_base: ::PhysicalAddress,
	frame_buffer_size: usize,
}

#[repr(C)]
pub struct BltPixel
{
	blue: u8,
	green: u8,
	red: u8,
	reserved: u8,
}

#[repr(C)]
pub enum BltOperation
{
	VideoFill,
	VideoToBltBuffer,
	BufferToVideo,
	VideoToVideo,
}

