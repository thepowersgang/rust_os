use crate::metadevs::video::bootvideo::VideoMode;

extern crate uefi_proto;
pub use self::uefi_proto::MAGIC;

pub struct UefiParsed
{
	pub cmdline: &'static str,
	pub vidmode: Option<VideoMode>,
	pub memmap: &'static [crate::memory::MemoryMapEnt],
}

impl UefiParsed
{
	pub unsafe fn from_ptr(info_ptr: *const crate::Void, mmap_buf: &'static mut [crate::memory::MemoryMapEnt]) -> Option<Self>
	{
		let info = &*(info_ptr as *const uefi_proto::Info);
		log_trace!("info = {:p}", info);
		let mut ret = UefiParsed {
				cmdline: Self::_cmdline(info),
				vidmode: None,//MultibootParsed::_vidmode(info),
				//symbol_info: MultibootParsed::_syminfo(info),
				memmap: &[],
			};
		// - Memory map is initialised afterwards so it gets easy access to used addresses
		ret.memmap = ret.fill_memmap(info, mmap_buf);
		Some( ret )
	}

	fn _cmdline(info: &uefi_proto::Info) -> &'static str {
		// SAFE: We can't easily check, so trust the bootloader
		unsafe {
			::core::str::from_utf8( ::core::slice::from_raw_parts(info.cmdline_ptr, info.cmdline_len) ).expect("UefiParsed::_cmdline")
		}
	}
	fn fill_memmap<'a>(&self, info: &uefi_proto::Info, buf: &'a mut [crate::memory::MemoryMapEnt]) -> &'a [crate::memory::MemoryMapEnt] {
		// TODO: Put this elsewhere
		struct StrideSlice<T> {
			ptr: *const T,
			count: usize,
			stride: usize,
		}
		impl<T> StrideSlice<T> {
			unsafe fn new(ptr: *const T, count: usize, stride: usize) -> StrideSlice<T> {
				StrideSlice {
					ptr: ptr, count: count, stride: stride
					}
			}
			//fn get(&self, idx: usize) -> &T {
			//	assert!(idx < self.count);
			//	let ptr = (self.ptr as usize + self.stride * idx) as *const T;
			//	// SAFE: Ensured by constructor and range checks
			//	unsafe { &*ptr }
			//}
		}
		impl<T: Copy> Iterator for StrideSlice<T> {
			type Item = T;
			fn next(&mut self) -> Option<T> {
				if self.count == 0 {
					None
				}
				else {
					// SAFE: Ensured by constructor and range checks
					let val = unsafe { *self.ptr };
					self.ptr = (self.ptr as usize + self.stride) as *const _;
					self.count -= 1;
					Some(val)
				}
			}
		}

		let size = {
			let /*mut*/ mapbuilder = crate::memory::MemoryMapBuilder::new(buf);
			// SAFE: Trusting the bootloader
			for ent in unsafe { StrideSlice::new(info.map_addr as *const uefi_proto::MemoryDescriptor, info.map_entnum as usize, info.map_entsz as usize) }
			{
				log_debug!("ent = {{ {}, {:#x}+{:#x} {:#x}", ent.ty, ent.physical_start, ent.number_of_pages, ent.attribute);
			}
			mapbuilder.size()
			};

		&buf[0 .. size]
	}
}