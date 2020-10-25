// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// load/mod.rs
// - Executable loading module
use std::io::{Read};

pub struct Segment {
	pub load_addr: usize,
	pub file_addr: u64,
	pub file_size: usize,
	pub mem_size: usize,
	pub protection: SegmentProt,
}
#[derive(Debug)]
pub enum SegmentProt {
	Execute,
	ReadOnly,
	ReadWrite,
}

//pub trait Executable<F: Read+Seek>
//{
//	type LoadSegments: SegmentIterator<F>;
//	fn get_entrypoint(&self) -> usize;
//	fn load_segments(&mut self) -> Self::LoadSegments;
//	fn do_relocation(&mut self) -> Result<(),()>;
//}

pub trait SegmentIterator<F: Read>:
	::std::iter::Iterator<Item=Segment>
{
	fn get_file(&self) -> &F;
}

impl_fmt! {
	Debug(self, f) for Segment {
		write!(f, "Segment {{ {:#x}+{:#x} <= {:#x}+{:#x} {:?} }}",
			self.load_addr, self.mem_size,
			self.file_addr, self.file_size,
			self.protection
			)
	}
}


/// Look up a symbol in the global symbol namespace
///
/// TODO: Needs support for weak symbols, and multiple namespaces (or preferential namespaces)
pub fn lookup_symbol(name: &::std::ffi::OsStr) -> Option<(usize, usize)> {
	match name.as_bytes()
	{
 	#[cfg(not(arch="native"))]
	b"new_process" => Some( (::interface::new_process as usize, 0) ),
 	#[cfg(not(arch="native"))]
	b"start_process" => Some( (::interface::start_process as usize, 0) ),
	_ => todo!("lookup_symbol({:?})", name),
	}
}

