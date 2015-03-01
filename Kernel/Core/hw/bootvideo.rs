// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/bootvideo.rs
// - Bootloader-provided video handling (simple framebuffer)
module_define!(BootVideo, [Video], init);

#[derive(Copy,Debug)]
pub enum VideoFormat
{
	X8R8G8B8,
	B8G8R8X8,
	R8G8B8,
	B8G8R8,
	R5G6B5,
}

#[derive(Copy,Debug)]
pub struct VideoMode
{
	pub width: u16,
	pub height: u16,
	pub fmt: VideoFormat,
	pub pitch: usize,
	pub base: ::arch::memory::PAddr,
}



fn init()
{
}

pub fn register(mode: VideoMode)
{
	log_error!("TODO: register(mode={:?})", mode);
}


