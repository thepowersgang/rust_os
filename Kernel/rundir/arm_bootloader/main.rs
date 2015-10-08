//
//
//
#![no_std]
#![feature(no_std,lang_items)]
#![feature(core_str_ext,core_slice_ext)]


/// Stub logging macro
macro_rules! log{
	($($v:tt)*) => {{
		use core::fmt::Write;
		let mut lh = ::Logger;
		let _ = write!(lh, "[loader log] ");
		let _ = write!(lh, $($v)*);
		let _ = write!(lh, "\n");
		}};
}

type Elf32_Half = u16;
type Elf32_Addr = u32;
type Elf32_Off = u32;
type Elf32_Sword = i32;
type Elf32_Word = u32;

#[repr(C)]
struct ElfHeader {
	e_ident: [u8; 16],
	e_object_type: Elf32_Half,
	e_machine_type: Elf32_Half,
	e_version: Elf32_Word,

	e_entry: Elf32_Addr,
	e_phoff: Elf32_Off,
	e_shoff: Elf32_Off,

	e_flags: Elf32_Word,
	e_ehsize: Elf32_Half,

	e_phentsize: Elf32_Half,
	e_phnum: Elf32_Half,

	e_shentsize: Elf32_Half,
	e_shnum: Elf32_Half,
	e_shstrndx: Elf32_Half,
}
#[repr(C)]
#[derive(Copy,Clone)]
struct PhEnt {
	p_type: Elf32_Word,
	p_offset: Elf32_Off,
	p_vaddr: Elf32_Addr,
	p_paddr: Elf32_Addr,	// aka load
	p_filesz: Elf32_Word,
	p_memsz: Elf32_Word,
	p_flags: Elf32_Word,
	p_align: Elf32_Word,
}
pub struct ElfFile(ElfHeader);
impl ElfFile
{
	pub fn check_header(&self) {
		assert_eq!(&self.0.e_ident[..8], b"\x7FELF\x01\x01\x01\x00");	// Elf32, LSB, Version, Pad
		assert_eq!(self.0.e_version, 1);
	}
	fn phents(&self) -> PhEntIter {
		assert_eq!( self.0.e_phentsize as usize, ::core::mem::size_of::<PhEnt>() );
		// SAFE: Assuming the file is correct...
		let slice: &[PhEnt] = unsafe {
			let ptr = (&self.0 as *const _ as usize + self.0.e_phoff as usize) as *const PhEnt;
			::core::slice::from_raw_parts( ptr, self.0.e_phnum as usize )
			};
		log!("phents() - slice = {:p}+{}", slice.as_ptr(), slice.len());
		PhEntIter( slice )
	}

	pub fn entrypoint(&self) -> usize {
		self.0.e_entry as usize
	}
}
struct PhEntIter<'a>(&'a [PhEnt]);
impl<'a> Iterator for PhEntIter<'a> {
	type Item = PhEnt;
	fn next(&mut self) -> Option<PhEnt> {
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
	max_end as u32
}

#[no_mangle]
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

#[no_mangle]
pub extern "C" fn elf_load_symbols(file_base: &ElfFile, output_base: *mut u8) -> u32
{
	log!("elf_load_symbols(file_base={:p}, output_base={:p})", file_base, output_base);
	0
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
