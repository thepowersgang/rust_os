//
//
//
#![no_std]
#![feature(lang_items)]

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
macro_rules! log{
	($($v:tt)*) => {{
		::log_closure(|lh| {let _ = write!(lh, $($v)*);});
		}};
}


pub struct ElfFile(elf_fmt::ElfHeader);
impl ElfFile
{
	pub fn check_header(&self) {
		assert_eq!(&self.0.e_ident[..8], b"\x7FELF\x01\x01\x01\x00");	// Elf32, LSB, Version, Pad
		assert_eq!(self.0.e_version, 1);
	}
	fn phents(&self) -> PhEntIter {
		assert_eq!( self.0.e_phentsize as usize, ::core::mem::size_of::<elf_fmt::PhEnt>() );
		// SAFE: Assuming the file is correct...
		let slice: &[elf_fmt::PhEnt] = unsafe {
			let ptr = (&self.0 as *const _ as usize + self.0.e_phoff as usize) as *const elf_fmt::PhEnt;
			::core::slice::from_raw_parts( ptr, self.0.e_phnum as usize )
			};
		log!("phents() - slice = {:p}+{}", slice.as_ptr(), slice.len());
		PhEntIter( slice )
	}
	fn shents(&self) -> &[elf_fmt::ShEnt] {
		assert_eq!( self.0.e_shentsize as usize, ::core::mem::size_of::<elf_fmt::ShEnt>() );
		// SAFE: Assuming the file is correct...
		unsafe {
			let ptr = (&self.0 as *const _ as usize + self.0.e_shoff as usize) as *const elf_fmt::ShEnt;
			::core::slice::from_raw_parts( ptr, self.0.e_shnum as usize )
		}
	}

	pub fn entrypoint(&self) -> usize {
		self.0.e_entry as usize
	}
}
struct PhEntIter<'a>(&'a [elf_fmt::PhEnt]);
impl<'a> Iterator for PhEntIter<'a> {
	type Item = elf_fmt::PhEnt;
	fn next(&mut self) -> Option<elf_fmt::PhEnt> {
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
//struct ShEntIter<'a>(&'a [elf_fmt::ShEnt]);
//impl<'a> Iterator for ShEntIter<'a> {
//	type Item = elf_fmt::ShEnt;
//	fn next(&mut self) -> Option<elf_fmt::ShEnt> {
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
pub extern "C" fn elf_get_size(file_base: &ElfFile) -> u32
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
	max_end as u32
}

#[no_mangle]
/// Returns program entry point
pub extern "C" fn elf_load_segments(file_base: &ElfFile, output_base: *mut u8) -> u32
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
	

	let rv = (file_base.entrypoint() - 0x80000000 + output_base as usize) as u32;
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
pub extern "C" fn elf_load_symbols(file_base: &ElfFile, output: &mut SymbolInfo) -> u32
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

			output.base = (output as *const _ as usize + pos) as *const _;
			output.count = ent.sh_size as usize / ::core::mem::size_of::<elf_fmt::SymEnt>();
			unsafe {
				let bytes = ent.sh_size as usize;
				let src = ::core::slice::from_raw_parts( (file_base as *const _ as usize + ent.sh_offset as usize) as *const elf_fmt::SymEnt, output.count );
				let dst = ::core::slice::from_raw_parts_mut( output.base as *mut elf_fmt::SymEnt, output.count );
				for (d,s) in Iterator::zip( dst.iter_mut(), src.iter() ) {
					//log!("- {:?} = {:#x}+{:#x}", ::core::str::from_utf8(&strtab_bytes[s.st_name as usize..].split(|&v|v==0).next().unwrap()), s.st_value, s.st_size);
					*d = *s;
				}
				pos += bytes;
			}
			output.string_table = (output as *const _ as usize + pos) as *const _;
			output.strtab_len = strtab.sh_size as usize;
			unsafe {
				let bytes = strtab.sh_size as usize;
				let src = ::core::slice::from_raw_parts( (file_base as *const _ as usize + strtab.sh_offset as usize) as *const u8, bytes );
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
	pos as u32
}



//
//
//

#[lang="eh_personality"]
fn eh_personality() -> ! {
	loop {}
}
#[lang="panic_fmt"]
fn panic_fmt() -> ! {
	loop {}
}


extern "C" {
	fn puts(_: *const u8, _: u32);
}
struct Logger;
impl ::core::fmt::Write for Logger {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result
	{
		// SAFE: Single-threaded
		unsafe {
			puts(s.as_ptr(), s.len() as u32);
		}
		Ok( () )
	}
}
