// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// This program is both the initial entrypoint for the userland, and the default dynamic linker.
#![feature(result_expect)]	// my feature, i'm using it
#![feature(core)]	// needed for core's SliceExt
#![crate_type="lib"]
#[macro_use]
extern crate tifflin_syscalls;

extern crate byteorder;
extern crate cmdline_words_parser;


macro_rules! impl_from {
	($(From<$src:ty>($v:ident) for $t:ty { $($code:stmt)*} )+) => {
		$(
			impl ::std::convert::From<$src> for $t {
				fn from($v: $src) -> $t {
					$($code)*
				}
			}
		)+
	}
}

use cmdline_words_parser::StrExt;

mod elf;

// Main: This is the initial boot entrypoint
#[no_mangle]
pub extern "C" fn loader_main(cmdline: *mut u8, cmdline_len: usize) -> !
{
	kernel_log!("loader_main({:p}, {})", cmdline, cmdline_len);
	// (maybe) SAFE: Need to actually check UTF-8 ness?
	let cmdline: &mut str = unsafe { ::std::mem::transmute( ::std::slice::from_raw_parts_mut(cmdline, cmdline_len) ) };
	// 1. Print the INIT parameter from the kernel
	kernel_log!("- cmdline={:?}", cmdline);
	
	// 2. Parse 'cmdline' into the init path and arguments.
	// TODO: Parse path as an escaped string. Should be able to use a parser that takes &mut str and returns reborrows
	//       - Such a parser would be able to clobber the string as escaping is undone, using the assumption that esc.len >= real.len
	let mut arg_iter = cmdline.parse_cmdline_words();
	let init_path = arg_iter.next().expect("Init path is empty");
	kernel_log!("- init_path={:?}", init_path);
	// 3. Spin up init
	// - Open the init path passed in `cmdline`
	let mut handle = match ::elf::load_executable(init_path)
		{
		Ok(v) => v,
		Err(e) => {
			kernel_log!("ERROR: Init binary '{}' cannot be loaded: {:?}", init_path, e);
			::tifflin_syscalls::exit(0);
			},
		};
	
	// I would love to use a for loop here, but getting access the file is hard using that
	{
		let mut segments_it = handle.load_segments();
		while let Some(segment) = segments_it.next()
		{
			use tifflin_syscalls::vfs::MemoryMapMode;
			use tifflin_syscalls::memory::ProtectionMode;
			const PAGE_SIZE: usize = 0x1000;
			kernel_log!("segment = {:?}", segment);
			
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
				elf::SegmentProt::Execute   => MemoryMapMode::Execute,
				elf::SegmentProt::ReadWrite => MemoryMapMode::COW,
				elf::SegmentProt::ReadOnly  => MemoryMapMode::ReadOnly,
				};
			let prot_mode = match segment.protection
				{
				elf::SegmentProt::Execute   => ProtectionMode::Executable,
				elf::SegmentProt::ReadWrite => ProtectionMode::CopyOnWrite,
				elf::SegmentProt::ReadOnly  => ProtectionMode::ReadOnly,
				};
			let fp = segments_it.get_file();
			if aligned > 0 {
				fp.memory_map(segment.file_addr, aligned, segment.load_addr, map_mode);
			}
			if tail > 0 {
				unsafe {
					let destslice = ::std::slice::from_raw_parts_mut((segment.load_addr + aligned) as *mut u8, tail);
					::tifflin_syscalls::memory::allocate(destslice.as_ptr() as usize, ProtectionMode::ReadWrite);
					fp.read_at(segment.file_addr + aligned as u64, destslice).expect("TODO");
					::tifflin_syscalls::memory::reprotect(destslice.as_ptr() as usize, prot_mode);
				}
			}
			if extra > PAGE_SIZE - tail {
				panic!("TODO: Allocate extra pages for BSS");
			}
		}
	}
	
	// Populate arguments
	// SAFE: We will be writing to this before reading from it
	let mut args_buf: [&str; 16] = unsafe { ::std::mem::uninitialized() };
	let mut argc = 0;
	for arg in arg_iter {
		args_buf[argc] = arg;
		argc += 1;
	}
	let args = &args_buf[..argc];
	kernel_log!("args = {:?}", args);
	
	// TODO: Switch stacks into a larger dynamically-allocated stack
	let ep: fn(&[&str]) -> ! = handle.get_entrypoint();
	kernel_log!("Calling entry {:p}", ep as *const ());
	ep(args);
	
	::tifflin_syscalls::exit(0);
}
