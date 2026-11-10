// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// This program is both the initial entrypoint for the userland, and the default dynamic linker.

use load::SegmentIterator;

#[link(name="loader_start")]
extern "C" {
}

#[macro_use]
extern crate syscalls;

extern crate byteorder;
extern crate cmdline_words_parser;

#[macro_use(impl_from, impl_fmt, todo)]
extern crate macros;

#[cfg(not(arch="native"))]	// Implemented elsewhere for native
pub mod interface;

mod elf;
mod load;
mod fixed_vec;

use self::fixed_vec::FixedVec;
use ::syscalls::PAGE_SIZE;

// Main: This is the initial boot entrypoint
// NOTE: If you're looking for the new process entrypoint, see interface.rs
#[no_mangle]
#[cfg(not(building_loader_lib))]
pub extern "C" fn loader_main(cmdline: *mut u8, cmdline_len: usize) -> !
{
	kernel_log!("loader_main({:p}, {})", cmdline, cmdline_len);
	// SAFE: (barring bugs in caller)
	let cmdline: &mut [u8] = unsafe { ::std::slice::from_raw_parts_mut(cmdline, cmdline_len) };

	// 0. Generate a guard page (by deallocating a special guard page just before the stack)
	// SAFE: The memory freed is reserved explicitly for use as a guard page
	/*unsafe {
		extern "C" {
			static init_stack_base: [u8; 0];
		}
		let _ = ::syscalls::memory::deallocate( (init_stack_base.as_ptr() as usize) - ::PAGE_SIZE );
	}
	*/
	
	// 1. Print the INIT parameter from the kernel
	kernel_log!("- cmdline={:?}", ::std::ffi::OsStr::new(&cmdline));
	
	// 2. Parse 'cmdline' into the init path and arguments.
	let mut arg_iter = ::cmdline_words_parser::parse_posix(cmdline).map(|v| ::std::ffi::OsStr::new(v));
	let init_path = arg_iter.next().expect("Init path is empty");
	kernel_log!("- init_path={:?}", init_path);
	
	
	// 3. Spin up init
	let fh: ::syscalls::vfs::File = ::syscalls::object_from_raw(2).expect("Unable to open object #2 as init");
	let entrypoint = load_binary(init_path, fh);
	
	// Populate arguments
	let mut args = FixedVec::new();
	args.push(init_path).unwrap();
	for arg in arg_iter {
		args.push(arg).unwrap();
	}
	kernel_log!("args = {:?}", &*args);
	
	// TODO: Switch stacks into a larger dynamically-allocated stack
	// SAFE: Entrypoint assumed to have this format... will likely crash if it isn't
	let ep: fn(&[&::std::ffi::OsStr]) = unsafe { ::std::mem::transmute(entrypoint) };
	kernel_log!("Calling entry {:p} for INIT {:?}", ep as *const (), init_path);
	ep(&args);
	kernel_log!("User entrypoint returned");
	::syscalls::threads::exit(!0);
}

/// Panics if it fails to load, returns the entrypoint
fn load_binary(path: &::std::ffi::OsStr, fh: ::syscalls::vfs::File) -> usize
{
	kernel_log!("load_binary({:?})", path);
	// - Open the init path passed in `cmdline`
	let mut handle = match ::elf::load_executable(fh)
		{
		Ok(v) => v,
		Err(e) => {
			panic!("ERROR: Binary '{:?}' cannot be loaded: {:?}", ::std::ffi::OsStr::new(path), e);
			},
		};
	
	let entrypoint = handle.get_entrypoint();
	kernel_log!("- entrypoint = {:#x}", entrypoint);
	
	let mut found_segment_for_entry = false;
	// I would love to use a for loop here, but getting access the file is hard using that
	{
		let mut segments_it = handle.load_segments();
		while let Some(segment) = segments_it.next()
		{
			use syscalls::vfs::MemoryMapMode;
			use syscalls::memory::ProtectionMode;
			kernel_log!("segment = {:?}", segment);
			
			if segment.load_addr <= entrypoint && entrypoint < segment.load_addr + segment.mem_size {
				found_segment_for_entry = true;
			}
			assert!(segment.load_addr & (PAGE_SIZE -1) == 0, "Unaligned segment base {:?}", segment);
			
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
				let mm = fp.memory_map(segment.file_addr, aligned, segment.load_addr as *mut _, map_mode);
				::std::mem::forget(mm);
			}
			if tail > 0 {
				assert!(aligned % PAGE_SIZE == 0);
				// SAFE: Trusing addresses to be valid
				unsafe {
					let destslice = ::std::slice::from_raw_parts_mut((segment.load_addr + aligned) as *mut u8, tail);
					// - Allocate space
					::syscalls::memory::allocate(destslice.as_ptr() as usize, 1).expect("tail alloc");
					// - Read data
					fp.read_at(segment.file_addr + aligned as u64, destslice).expect("Failure reading file data for end of .segment");
					// - Reprotect to the real mode, not bothering if the desired is Read-Write
					if alloc_mode != ProtectionMode::ReadWrite {
						::syscalls::memory::reprotect(destslice.as_ptr() as usize, alloc_mode).expect("reprotect");
					}
				}
			}
			if extra > PAGE_SIZE - tail {
				let addr = (segment.load_addr + aligned + PAGE_SIZE) & !(PAGE_SIZE-1);
				let pages = (extra - (PAGE_SIZE - tail) + PAGE_SIZE-1) / PAGE_SIZE;
				// SAFE: Just allocating at a known free place
				unsafe { ::syscalls::memory::allocate(addr, pages).expect("extra alloc"); }
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

