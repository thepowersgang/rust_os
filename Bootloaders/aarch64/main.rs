//
//
//
#![no_std]
#![feature(lang_items)]
#![feature(panic_info_message)]

mod elf_fmt;

#[inline(never)]
fn log_closure<F: FnOnce(&mut ::core::fmt::Write)>(f: F) {
	use core::fmt::Write;
	let mut lh = ::Logger;
	let _ = write!(lh, "[loader log] ");
	f(&mut lh);
	let _ = write!(lh, "\n");
}

/// Stub logging macro
macro_rules! log {
	($($v:tt)*) => {{
		::log_closure(|lh| {let _ = write!(lh, $($v)*);});
		}};
}


pub struct ElfFile(elf_fmt::ElfHeader);
impl ElfFile
{
	pub fn check_header(&self) {
		assert_eq!(&self.0.e_ident[..8], b"\x7FELF\x02\x01\x01\x00");	// Elf64, LSB, Version, Pad
		assert_eq!(self.0.e_version, 1);
	}
	fn phents(&self) -> PhEntIter {
		assert_eq!( self.0.e_phentsize as usize, ::core::mem::size_of::<elf_fmt::Elf64_PhEnt>() );
		// SAFE: Assuming the file is correct...
		let slice: &[elf_fmt::Elf64_PhEnt] = unsafe {
			let ptr = (&self.0 as *const _ as usize + self.0.e_phoff as usize) as *const elf_fmt::Elf64_PhEnt;
			::core::slice::from_raw_parts( ptr, self.0.e_phnum as usize )
			};
		log!("phents() - slice = {:p}+{}", slice.as_ptr(), slice.len());
		PhEntIter( slice )
	}
	fn shents(&self) -> &[elf_fmt::Elf64_ShEnt] {
		assert_eq!( self.0.e_shentsize as usize, ::core::mem::size_of::<elf_fmt::Elf64_ShEnt>() );
		// SAFE: Assuming the file is correct...
		unsafe {
			let ptr = (&self.0 as *const _ as usize + self.0.e_shoff as usize) as *const elf_fmt::Elf64_ShEnt;
			::core::slice::from_raw_parts( ptr, self.0.e_shnum as usize )
		}
	}

	pub fn entrypoint(&self) -> usize {
		self.0.e_entry as usize
	}
}
struct PhEntIter<'a>(&'a [elf_fmt::Elf64_PhEnt]);
impl<'a> Iterator for PhEntIter<'a> {
	type Item = elf_fmt::Elf64_PhEnt;
	fn next(&mut self) -> Option<elf_fmt::Elf64_PhEnt> {
		if self.0.len() == 0 {
			None
		}
		else {
			let rv = self.0[0].clone();
			self.0 = &self.0[1..];
			Some(rv)
		}
	}
}
//struct ShEntIter<'a>(&'a [elf_fmt::Elf64_ShEnt]);
//impl<'a> Iterator for ShEntIter<'a> {
//	type Item = elf_fmt::Elf64_ShEnt;
//	fn next(&mut self) -> Option<elf_fmt::Elf64_ShEnt> {
//		if self.0.len() == 0 {
//			None
//		}
//		else {
//			let rv = self.0[0].clone();
//			self.0 = &self.0[1..];
//			Some(rv)
//		}
//	}
//}

#[no_mangle]
pub extern "C" fn elf_get_size(file_base: &ElfFile) -> usize
{
	log!("elf_get_size(file_base={:p})", file_base);
	file_base.check_header();

	let mut max_end = 0;
	for phent in file_base.phents()
	{
		if phent.p_type == 1
		{
			log!("- {:#x}+{:#x} loads +{:#x}+{:#x}",
				phent.p_paddr, phent.p_memsz,
				phent.p_offset, phent.p_filesz
				);
			
			let end = (phent.p_paddr + phent.p_memsz) as usize;
			if max_end < end {
				max_end = end;
			}
		}
	}
	// Round the image size to 4KB
	let max_end = (max_end + 0xFFF) & !0xFFF;
	log!("return load_size={:#x}", max_end);
	if max_end == 0 {
		log!("ERROR!!! Kernel reported zero loadable size");
		loop {}
	}
	max_end
}

#[no_mangle]
/// Returns program entry point
pub extern "C" fn elf_load_segments(file_base: &ElfFile, output_base: *mut u8) -> usize
{
	log!("elf_load_segments(file_base={:p}, output_base={:p})", file_base, output_base);
	for phent in file_base.phents()
	{
		if phent.p_type == 1
		{
			log!("- {:#x}+{:#x} loads +{:#x}+{:#x}",
				phent.p_paddr, phent.p_memsz,
				phent.p_offset, phent.p_filesz
				);
			
			let (dst,src) = unsafe {
				let dst = ::core::slice::from_raw_parts_mut( (output_base as usize + phent.p_paddr as usize) as *mut u8, phent.p_memsz as usize );
				let src = ::core::slice::from_raw_parts( (file_base as *const _ as usize + phent.p_offset as usize) as *const u8, phent.p_filesz as usize );
				(dst, src)
				};
			for (d, v) in Iterator::zip( dst.iter_mut(), src.iter().cloned().chain(::core::iter::repeat(0)) )
			{
				*d = v;
			}
		}
	}
	
	const KERNEL_VBASE: usize = 0xFFFF800000000000;
	let rv = file_base.entrypoint() - KERNEL_VBASE + output_base as usize;
	log!("return entrypoint={:#x}", rv);
	rv
}

#[repr(C)]
#[derive(Debug)]
pub struct SymbolInfo {
	base: *const elf_fmt::SymEnt,
	count: usize,
	string_table: *const u8,
	strtab_len: usize,
}

#[no_mangle]
/// Returns size of data written to output_base
pub extern "C" fn elf_load_symbols(file_base: &ElfFile, output: &mut SymbolInfo) -> usize
{
	log!("elf_load_symbols(file_base={:p}, output={:p})", file_base, output);
	*output = SymbolInfo {base: 0 as *const _, count: 0, string_table: 0 as *const _, strtab_len: 0};
	let mut pos = ::core::mem::size_of::<SymbolInfo>();
	for ent in file_base.shents()
	{
		if ent.sh_type == 2
		{
			log!("Symbol table at +{:#x}+{:#x}, string table {}", ent.sh_offset, ent.sh_size, ent.sh_link);
			let strtab = file_base.shents()[ent.sh_link as usize];
			let strtab_bytes = unsafe { ::core::slice::from_raw_parts( (file_base as *const _ as usize + strtab.sh_offset as usize) as *const u8, strtab.sh_size as usize ) };
			//log!("- strtab = {:?}", ::core::str::from_utf8(strtab_bytes));

			// Copy symbol table
			output.base = (output as *const _ as usize + pos) as *const _;
			output.count = ent.sh_size as usize / ::core::mem::size_of::<elf_fmt::SymEnt>();
			unsafe {
				let bytes = ent.sh_size as usize;
				let src = ::core::slice::from_raw_parts( (file_base as *const _ as usize + ent.sh_offset as usize) as *const elf_fmt::SymEnt, output.count );
				let dst = ::core::slice::from_raw_parts_mut( output.base as *mut elf_fmt::Elf32_SymEnt, output.count );
				for (d,s) in Iterator::zip( dst.iter_mut(), src.iter() ) {
					//log!("- {:?} = {:#x}+{:#x}", ::core::str::from_utf8(&strtab_bytes[s.st_name as usize..].split(|&v|v==0).next().unwrap()), s.st_value, s.st_size);
					*d = elf_fmt::Elf32_SymEnt {
						st_name: s.st_name,
						st_value: s.st_value as u32,	// mask down
						st_size: s.st_size as u32,
						st_info: s.st_info,
						st_other: s.st_other,
						st_shndx: s.st_shndx,
						};
				}
				pos += bytes;
			}
			// Copy string table
			output.string_table = (output as *const _ as usize + pos) as *const _;
			output.strtab_len = strtab.sh_size as usize;
			unsafe {
				let bytes = strtab.sh_size as usize;
				let src = strtab_bytes;
				let dst = ::core::slice::from_raw_parts_mut( output.string_table as *mut u8, bytes );
				for (d,s) in Iterator::zip( dst.iter_mut(), src.iter() ) {
					*d = *s;
				}
				pos += bytes;
			}
			break ;
		}
	}

	log!("- output = {:?}", output);
	pos
}



//
//
//

#[lang="eh_personality"]
fn eh_personality() -> ! {
	puts("UNWIND");
	loop {}
}
fn begin_panic_fmt(msg: &::core::fmt::Arguments, file_line: (&str, u32)) -> ! {
	puts("PANIC @ "); puts(file_line.0); puts("\n");
	log!("panic: {}:{}: {}\n", file_line.0, file_line.1, msg);
	loop {}
}
#[panic_handler]
fn panic_handler(info: &::core::panic::PanicInfo) -> ! {
	let file_line = match info.location()
		{
		Some(v) => (v.file(), v.line()),
		None => ("", 0),
		};
	if let Some(m) = info.payload().downcast_ref::<::core::fmt::Arguments>() {
		begin_panic_fmt(m, file_line)
	}
	else if let Some(m) = info.payload().downcast_ref::<&str>() {
		begin_panic_fmt(&format_args!("{}", m), file_line)
	}
	else if let Some(m) = info.message() {
		begin_panic_fmt(m, file_line)
	}
	else {
		begin_panic_fmt(&format_args!("Unknown"), file_line)
	}
}


extern "C" {
	#[link_name="puts"]
	fn puts_raw(_: *const u8, _: u32);
}
fn puts(s: &str) {
	// SAFE: Single-threaded
	unsafe {
		puts_raw(s.as_ptr(), s.len() as u32);
	}
}

struct Logger;
impl ::core::fmt::Write for Logger {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result
	{
		puts(s);
		Ok( () )
	}
}
