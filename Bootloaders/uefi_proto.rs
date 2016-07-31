
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

