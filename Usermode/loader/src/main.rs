// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// This program is both the initial entrypoint for the userland, and the default dynamic linker.
#![feature(result_expect)]	// my feature, i'm using it
#![feature(core_slice_ext)]	// needed for core's SliceExt
#![crate_type="lib"]

use cmdline_words_parser::StrExt as CmdlineStrExt;

use load::SegmentIterator;

#[macro_use]
extern crate tifflin_syscalls;

extern crate byteorder;
extern crate cmdline_words_parser;

#[macro_use(impl_from, impl_fmt, try, todo)]
extern crate macros;

mod elf;
pub mod interface;
mod load;

// Main: This is the initial boot entrypoint
#[no_mangle]
#[cfg(not(building_loader_lib))]
pub extern "C" fn loader_main(cmdline: *mut u8, cmdline_len: usize) -> !
{
	kernel_log!("loader_main({:p}, {})", cmdline, cmdline_len);
	// SAFE: (barring bugs in caller) Transmute just keeps 'mut' on the OsStr
	let cmdline: &mut ::std::ffi::OsStr = unsafe { ::std::mem::transmute( ::std::slice::from_raw_parts_mut(cmdline, cmdline_len) ) };
	
	// 1. Print the INIT parameter from the kernel
	kernel_log!("- cmdline={:?}", cmdline);
	
	// 2. Parse 'cmdline' into the init path and arguments.
	let mut arg_iter = cmdline.parse_cmdline_words();
	let init_path = arg_iter.next().expect("Init path is empty");
	kernel_log!("- init_path={:?}", init_path);
	
	// 3. Spin up init
	let entrypoint = load_binary(init_path);
	
	// Populate arguments
	// TODO: Replace this mess with a FixedVec of some form
	// SAFE: We will be writing to this before reading from it
	let mut args_buf: [&::std::ffi::OsStr; 16] = unsafe { ::std::mem::uninitialized() };
	let mut argc = 0;
	args_buf[argc] = init_path;
	argc += 1;
	for arg in arg_iter {
		args_buf[argc] = arg;
		argc += 1;
	}
	let args = &args_buf[..argc];
	kernel_log!("args = {:?}", args);
	
	// TODO: Switch stacks into a larger dynamically-allocated stack
	let ep: fn(&[&::std::ffi::OsStr]) -> ! = unsafe { ::std::mem::transmute(entrypoint) };
	kernel_log!("Calling entry {:p}", ep as *const ());
	ep(args);
}

/// Panics if it fails to load, returns the entrypoint
fn load_binary(path: &::std::ffi::OsStr) -> usize
{
	// - Open the init path passed in `cmdline`
	let mut handle = match ::elf::load_executable(path)
		{
		Ok(v) => v,
		Err(e) => {
			panic!("ERROR: Binary '{:?}' cannot be loaded: {:?}", ::std::ffi::OsStr::new(path), e);
			},
		};
	
	let entrypoint = handle.get_entrypoint();
	
	let mut found_segment_for_entry = false;
	// I would love to use a for loop here, but getting access the file is hard using that
	{
		let mut segments_it = handle.load_segments();
		while let Some(segment) = segments_it.next()
		{
			use tifflin_syscalls::vfs::MemoryMapMode;
			use tifflin_syscalls::memory::ProtectionMode;
			const PAGE_SIZE: usize = 0x1000;
			kernel_log!("segment = {:?}", segment);
			
			if segment.load_addr <= entrypoint && entrypoint < segment.load_addr + segment.mem_size {
				found_segment_for_entry = true;
			}
			
			assert!(segment.file_size <= segment.mem_size);
			// Split the segment into three regions: (reverse)
			// - Page-aligned resident data
			// - Non-resident data
			// - Tailing resident data
			let extra = segment.mem_size - segment.file_size;
			let tail    = segment.file_size % PAGE_SIZE;
			let aligned = segment.file_size - tail;
			let map_mode = match segment.protection
				{
				::load::SegmentProt::Execute   => MemoryMapMode::Execute,
				::load::SegmentProt::ReadWrite => MemoryMapMode::COW,
				::load::SegmentProt::ReadOnly  => MemoryMapMode::ReadOnly,
				};
			let alloc_mode = match segment.protection
				{
				::load::SegmentProt::Execute   => ProtectionMode::Executable,
				::load::SegmentProt::ReadWrite => ProtectionMode::ReadWrite,	// Allocates as read-write
				::load::SegmentProt::ReadOnly  => ProtectionMode::ReadOnly,
				};
			let fp = segments_it.get_file();
			if aligned > 0 {
				let mm = fp.memory_map(segment.file_addr, aligned, segment.load_addr, map_mode);
				::std::mem::forget(mm);
			}
			if tail > 0 {
				unsafe {
					let destslice = ::std::slice::from_raw_parts_mut((segment.load_addr + aligned) as *mut u8, tail);
					// - Allocate space
					::tifflin_syscalls::memory::allocate(destslice.as_ptr() as usize, 1).expect("tail alloc");
					// - Read data
					fp.read_at(segment.file_addr + aligned as u64, destslice).expect("Failure reading file data for end of .segment");
					// - Reprotect to the real mode, not bothering if the desired is Read-Write
					if alloc_mode != ProtectionMode::ReadWrite {
						::tifflin_syscalls::memory::reprotect(destslice.as_ptr() as usize, alloc_mode).expect("reprotect");
					}
				}
			}
			if extra > PAGE_SIZE - tail {
				let addr = segment.load_addr + aligned + PAGE_SIZE;
				let pages = (extra - (PAGE_SIZE - tail) + PAGE_SIZE-1) / PAGE_SIZE;
				unsafe {
					::tifflin_syscalls::memory::allocate(addr, pages).expect("extra alloc");
				}
			}
		}
	}
	
	if !found_segment_for_entry {
		panic!("Entrypoint {:#x} is not located in a loaded segment", entrypoint);
	}
	
	match handle.do_relocation()
	{
	Ok(_) => {},
	Err(e) => {
		panic!("Error relocating executable: {:?}", e);
		},
	}

	// TODO: Have a cleaner way of handling this, than just forgetting the handle
	// - Probably unwrap the handle into a raw file handle - THEN forget that (or even store it)
	::std::mem::forget(handle);
		
	entrypoint
}

