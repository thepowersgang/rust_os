#![crate_type="lib"]
#![no_std]

pub mod archapi
{

pub enum VideoFormat
{
	VideoX8R8G8B8,
	VideoB8G8R8X8,
	VideoR8G8B8,
	VideoB8G8R8,
	VideoR5G6B5,
}

pub struct VideoMode
{
	pub width: u16,
	pub height: u16,
	pub fmt: VideoFormat,
}

}

// vim: ft=rust

