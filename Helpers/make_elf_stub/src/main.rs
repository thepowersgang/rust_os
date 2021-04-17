//
//
//
use ::elf_utilities::header;

fn main()
{
	use ::elf_utilities::header::{Machine,OSABI};
	let args: Vec<_> = ::std::env::args().collect();
	let args = {
		let mut opts = ::getopts::Options::new();
		opts.reqopt("o", "output", "Output filename", "PATH");
		opts.reqopt("a", "arch", "Architecture name", "NAME");
		opts.optflag("", "elf32", "Create a 32-bit ELF");
		opts.parse(&args[1..]).unwrap()
		};
	
	let outfile = args.opt_str("output").unwrap();
	let (is_32,machine, abi, flags) = match &args.opt_str("arch").unwrap()[..]
		{
		"riscv64" => (false, Machine::Any(0xF3), OSABI::None, 0),
		"amd64"   => (false, Machine::X8664, OSABI::None, 0),
		"armv7"   => (true , Machine::/*Arm*/Any(0x28), OSABI::SysV, 0x5000000),
		"armv8"   => (false, Machine::/*AArch64*/Any(0xb7), OSABI::None, 0),
		name @ _ => panic!("Unknown machine type: {:?}", name),
		};

	let symbols: Vec<_> = args.free.iter().map(|v| &v[..]).collect();
	//let symbols = [ "new_process", "start_process", ];
	// TODO: Get the machine type and symbol list from the command line
	//make_elf_stub(outfile.as_ref(), machine, &symbols);
	if is_32 {
		make_elf_stub_g::<Elf32>(outfile.as_ref(), machine, abi, flags, &symbols);
	}
	else {
		make_elf_stub_g::<Elf64>(outfile.as_ref(), machine, abi, flags, &symbols);
	}
}
/*
// TODO: Support ELF32 too
fn make_elf_stub(out_path: &std::path::Path, machine: header::Machine, symbols: &[&str])
{
	let (dynstr,dynstr_mapping) = make_string_table(&symbols);
	let (shstrtab,shstrtab_mapping) = make_string_table(&[
		".shstrtab",	// .shstrtab: Section table headers
		".dynsym",		// .dynsym: Symbol table for dynamic
		".dynstr",		// .dynstr: String table for dynamic
		]);

	let header = {
		let mut hdr = ::elf_utilities::header::Ehdr64::default();
		hdr.set_file_version(header::Version::Current);
		hdr.set_object_version(header::Version::Current);
		hdr.set_class(header::Class::Bit64);
		hdr.set_data(header::Data::LSB2);
		hdr.set_machine(machine);	// RISCV
		hdr.set_elf_type(header::Type::Dyn);

		hdr.e_phoff = 0;
		hdr.e_shoff = ::elf_utilities::header::Ehdr64::SIZE as _;
		hdr.e_shnum = 3;
		hdr.e_shentsize = ::elf_utilities::section::Shdr64::SIZE as _;
		hdr.e_shstrndx = 1;

		hdr
		};
	let mut section_table = [
		// Dynamic strings
		::elf_utilities::section::Shdr64 {
			sh_name: shstrtab_mapping[1],
			sh_addr: 0,
			sh_addralign: 0,
			sh_entsize: 1,
			sh_flags: ::elf_utilities::section::Flag::Alloc as _,
			sh_info: 0,
			sh_link: 0,
			sh_offset: 0,
			sh_size: dynstr.len() as _,
			sh_type: ::elf_utilities::section::Type::StrTab.into(),
			},
		// Section string table
		::elf_utilities::section::Shdr64 {
			sh_name: shstrtab_mapping[0],
			sh_addr: 0,
			sh_addralign: 0,
			sh_entsize: 1,
			sh_flags: ::elf_utilities::section::Flag::Alloc as _,
			sh_info: 0,
			sh_link: 0,
			sh_offset: 0,
			sh_size: shstrtab.len() as _,
			sh_type: ::elf_utilities::section::Type::StrTab.into(),
			},
		// Dynamic symbols
		::elf_utilities::section::Shdr64 {
			sh_name: shstrtab_mapping[2],
			sh_addr: 0,
			sh_addralign: 0,
			sh_entsize: ::elf_utilities::symbol::Symbol64::SIZE as _,
			sh_flags: ::elf_utilities::section::Flag::Alloc as _,
			sh_info: 0,
			sh_link: 0,
			sh_offset: 0,
			sh_size: (symbols.len() * ::elf_utilities::symbol::Symbol64::SIZE) as _,
			sh_type: ::elf_utilities::section::Type::SymTab.into(),
			},
		];
	let mut ofs = ::elf_utilities::header::Ehdr64::SIZE as usize + section_table.len() * ::elf_utilities::section::Shdr64::SIZE as usize;
	for ent in &mut section_table {
		ent.sh_offset = ofs as _;
		ofs += ent.sh_size as usize;
	}
	
	use std::io::Write;
	let mut fp = ::std::fs::File::create(out_path).expect("Can't open output");
	fp.write(&header.to_le_bytes()).unwrap();
	for s in &section_table {
		fp.write(&s.to_le_bytes()).unwrap();
	}
	fp.write(&dynstr).unwrap();
	fp.write(&shstrtab).unwrap();
	for name_ofs in dynstr_mapping {
		fp.write(&elf_utilities::symbol::Symbol64 {
				st_name: name_ofs as u32,
				st_info: 0,
				st_other: 0,
				st_shndx: 1,
				st_value: 0x1000,
				st_size: 0,
				symbol_name: String::new(),
			}.to_le_bytes()).unwrap();
	}
}
*/
fn make_elf_stub_g<T: ElfClass>(out_path: &std::path::Path, machine: header::Machine, abi: header::OSABI, flags: u32, symbols: &[&str])
{
	let (dynstr,dynstr_mapping) = make_string_table(&symbols);
	let (shstrtab,shstrtab_mapping) = make_string_table(&[
		".shstrtab",	// .shstrtab: Section table headers
		".dynsym",		// .dynsym: Symbol table for dynamic
		".dynstr",		// .dynstr: String table for dynamic
		]);

	let header = T::header(machine, abi, flags, /*e_shnum*/3, /*e_shstrndx*/1);
	let mut section_table = [
		// Dynamic strings
		T::shdr(shstrtab_mapping[1], ::elf_utilities::section::Type::StrTab, /*sh_entsize*/1, dynstr.len() as _, /*sh_link*/0),
		// Section string table
		T::shdr(shstrtab_mapping[0], ::elf_utilities::section::Type::StrTab, /*sh_entsize*/1, shstrtab.len() as _, /*sh_link*/0),
		// Dynamic symbols
		T::shdr(shstrtab_mapping[2], ::elf_utilities::section::Type::SymTab, /*sh_entsize*/T::SYMBOL_SIZE as _, symbols.len() * T::SYMBOL_SIZE, /*sh_link*/0/*dynstr*/),
		];
	let mut ofs = T::HEADER_SIZE + section_table.len() * T::SHDR_SIZE;
	for ent in &mut section_table {
		let len = T::shdr_set_offset(ent, ofs);
		ofs += len;
	}
	
	use std::io::Write;
	let mut fp = ::std::fs::File::create(out_path).expect("Can't open output");
	header.write_to(&mut fp).unwrap();
	for s in &section_table {
		s.write_to(&mut fp).unwrap();
	}
	fp.write(&dynstr).unwrap();
	fp.write(&shstrtab).unwrap();
	for name_ofs in dynstr_mapping {
		T::symbol(name_ofs, 0x1000).write_to(&mut fp).unwrap();
	}
}

trait ElfClass
{
	const HEADER_SIZE: usize;
	const SYMBOL_SIZE: usize;
	const SHDR_SIZE: usize;

	type Ehdr: WriteTo;
	type Shdr: WriteTo;
	type Symbol: WriteTo;

	fn header(machine: header::Machine, abi: header::OSABI, flags: u32, e_shnum: u16, e_shstrndx: u32) -> Self::Ehdr;
	fn shdr(sh_name: u32, ty: ::elf_utilities::section::Type, sh_entsize: u32, size: usize, link: u32) -> Self::Shdr;
	fn symbol(st_name: u32, value: usize) -> Self::Symbol;

	fn shdr_set_offset(shdr: &mut Self::Shdr, ofs: usize) -> usize;
}
trait WriteTo {
	fn write_to<T: ::std::io::Write>(&self, sink: &mut T) -> ::std::io::Result<()>;
}
struct Elf64;
mod elf64_impl {
	use ::elf_utilities::header::Ehdr64;
	use ::elf_utilities::section::Shdr64;
	use ::elf_utilities::symbol::Symbol64;
	use ::elf_utilities::header;
	use super::{Elf64,ElfClass,WriteTo};
	impl ElfClass for Elf64 {
		const HEADER_SIZE: usize = Ehdr64::SIZE as _;
		const SHDR_SIZE: usize = Shdr64::SIZE as _;
		const SYMBOL_SIZE: usize = Symbol64::SIZE as _;
		type Ehdr = Ehdr64;
		type Shdr = Shdr64;
		type Symbol = Symbol64;

		fn header(machine: ::elf_utilities::header::Machine, abi: header::OSABI, flags: u32, e_shnum: u16, e_shstrndx: u32) -> Self::Ehdr {
			let mut hdr = Ehdr64::default();
			hdr.set_file_version(header::Version::Current);
			hdr.set_object_version(header::Version::Current);
			hdr.set_class(header::Class::Bit64);
			hdr.set_data(header::Data::LSB2);
			hdr.set_machine(machine);	// RISCV
			hdr.set_elf_type(header::Type::Dyn);
			hdr.set_osabi(abi);

			hdr.e_flags = flags;
			hdr.e_phoff = 0;
			hdr.e_shoff = Ehdr64::SIZE as _;
			hdr.e_shnum = e_shnum;
			hdr.e_shentsize = Shdr64::SIZE as _;
			hdr.e_shstrndx = e_shstrndx as _;

			hdr
		}
		fn shdr(sh_name: u32, ty: ::elf_utilities::section::Type, sh_entsize: u32, size: usize, link: u32) -> Self::Shdr {
			Shdr64 {
				sh_name: sh_name,
				sh_addr: 0,
				sh_addralign: 0,
				sh_entsize: sh_entsize as _,
				sh_flags: ::elf_utilities::section::Flag::Alloc as _,
				sh_info: 0,
				sh_link: link,
				sh_offset: 0,
				sh_size: size as _,
				sh_type: ty.into(),
				}
		}
		fn symbol(st_name: u32, value: usize) -> Self::Symbol {
			let mut rv = Symbol64 {
				st_name: st_name,
				st_info: 0,
				st_other: 0,
				st_shndx: 1,
				st_value: value as _,
				st_size: 0,
				symbol_name: String::new(),
				};
			rv.set_info(::elf_utilities::symbol::Type::Func, ::elf_utilities::symbol::Bind::Global);
			rv
		}

		fn shdr_set_offset(shdr: &mut Self::Shdr, ofs: usize) -> usize {
			shdr.sh_offset = ofs as _;
			shdr.sh_size as _
		}
	}
	impl WriteTo for Ehdr64 {
		fn write_to<T: ::std::io::Write>(&self, sink: &mut T) -> ::std::io::Result<()> {
			sink.write(&self.to_le_bytes()).map(|_| ())
		}
	}
	impl WriteTo for Shdr64 {
		fn write_to<T: ::std::io::Write>(&self, sink: &mut T) -> ::std::io::Result<()> {
			sink.write(&self.to_le_bytes()).map(|_| ())
		}
	}
	impl WriteTo for Symbol64 {
		fn write_to<T: ::std::io::Write>(&self, sink: &mut T) -> ::std::io::Result<()> {
			sink.write(&self.to_le_bytes()).map(|_| ())
		}
	}
}
struct Elf32;
mod elf32_impl {
	use ::elf_utilities::header::Ehdr32;
	use ::elf_utilities::section::Shdr32;
	use ::elf_utilities::symbol::Symbol32;
	use ::elf_utilities::header;
	use super::{Elf32,ElfClass,WriteTo};
	impl ElfClass for Elf32 {
		const HEADER_SIZE: usize = Ehdr32::SIZE as _;
		const SHDR_SIZE: usize = Shdr32::SIZE as _;
		const SYMBOL_SIZE: usize = Symbol32::SIZE as _;
		type Ehdr = Ehdr32;
		type Shdr = Shdr32;
		type Symbol = Symbol32;

		fn header(machine: ::elf_utilities::header::Machine, abi: header::OSABI, flags: u32, e_shnum: u16, e_shstrndx: u32) -> Self::Ehdr {
			let mut hdr = Ehdr32::default();
			hdr.set_file_version(header::Version::Current);
			hdr.set_object_version(header::Version::Current);
			hdr.set_class(header::Class::Bit32);
			hdr.set_data(header::Data::LSB2);
			hdr.set_machine(machine);	// RISCV
			hdr.set_elf_type(header::Type::Dyn);
			hdr.set_osabi(abi);

			hdr.e_flags = flags;
			hdr.e_phoff = 0;
			hdr.e_shoff = Ehdr32::SIZE as _;
			hdr.e_shnum = e_shnum;
			hdr.e_shentsize = Shdr32::SIZE as _;
			hdr.e_shstrndx = e_shstrndx as _;

			hdr
		}
		fn shdr(sh_name: u32, ty: ::elf_utilities::section::Type, sh_entsize: u32, size: usize, link: u32) -> Self::Shdr {
			Shdr32 {
				sh_name: sh_name,
				sh_addr: 0,
				sh_addralign: 0,
				sh_entsize: sh_entsize as _,
				sh_flags: ::elf_utilities::section::Flag::Alloc as _,
				sh_info: 0,
				sh_link: link,
				sh_offset: 0,
				sh_size: size as _,
				sh_type: ty.into(),
				}
		}
		fn symbol(st_name: u32, value: usize) -> Self::Symbol {
			let mut rv = Symbol32 {
				st_name: st_name,
				st_info: 0,
				st_other: 0,
				st_shndx: 1,
				st_value: value as _,
				st_size: 0,
				symbol_name: String::new(),
				};
			rv.set_info(::elf_utilities::symbol::Type::Func, ::elf_utilities::symbol::Bind::Global);
			rv
		}

		fn shdr_set_offset(shdr: &mut Self::Shdr, ofs: usize) -> usize {
			shdr.sh_offset = ofs as _;
			shdr.sh_size as _
		}
	}
	impl WriteTo for Ehdr32 {
		fn write_to<T: ::std::io::Write>(&self, sink: &mut T) -> ::std::io::Result<()> {
			sink.write(&self.to_le_bytes()).map(|_| ())
		}
	}
	impl WriteTo for Shdr32 {
		fn write_to<T: ::std::io::Write>(&self, sink: &mut T) -> ::std::io::Result<()> {
			sink.write(&self.to_le_bytes()).map(|_| ())
		}
	}
	impl WriteTo for Symbol32 {
		fn write_to<T: ::std::io::Write>(&self, sink: &mut T) -> ::std::io::Result<()> {
			sink.write(&self.to_le_bytes()).map(|_| ())
		}
	}
}

fn make_string_table(strings: &[&str]) -> (Vec<u8>, Vec<u32>) {
	let mut mappings = Vec::with_capacity(strings.len());
	let mut strtab: Vec<u8> = Vec::new();
	strtab.push(0);
	for sym in strings {
		mappings.push( strtab.len() as u32 );
		strtab.extend( sym.as_bytes().iter().copied() );
		strtab.push(0);
	}
	(strtab, mappings)
}

