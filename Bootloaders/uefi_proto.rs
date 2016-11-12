
#[repr(C)]
pub struct Info
{
	pub runtime_services: *const (),

	pub cmdline_ptr: *const u8,
	pub cmdline_len: usize,

	pub map_addr: u64,
	pub map_entnum: u32,
	pub map_entsz: u32,
}

// TODO: Grab this from libuefi
#[repr(C)]
#[derive(Copy,Clone)]
pub struct MemoryDescriptor
{
	pub ty: u32,
	_pad: u32,
	pub physical_start: u64,
	pub virtual_start: u64,
	pub number_of_pages: u64,
	pub attribute: u64,
	_pad2: u64,
}
