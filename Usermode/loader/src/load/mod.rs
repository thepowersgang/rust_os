// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// load/mod.rs
// - Executable loading module
use std::io::{Read,Seek,SeekFrom};

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

pub trait Executable<F: Read+Seek>
{
	type LoadSegments: SegmentIterator<F>;
	fn get_entrypoint(&self) -> usize;
	fn load_segments(&mut self) -> Self::LoadSegments;
	fn do_relocation(&mut self) -> Result<(),()>;
}

pub trait SegmentIterator<F: Read+Seek>:
	::std::iter::Iterator<Item=Segment>
{
	fn get_file(&self) -> &F;
}

impl_fmt! {
	Debug(self, f) for Segment {
		write!(f, "Segment {{ {:#x}+{:#x} <= {:#x}+{:#x} {:?} }}",
			self.load_addr, self.mem_size,
			self.file_size, self.file_size,
			self.protection
			)
	}
}
