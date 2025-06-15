// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// elf/mod.rs
// - ELF Parser

use syscalls::vfs::File;
use syscalls::vfs::Error as VfsError;

use std::io::{Read,Seek,SeekFrom};

use load::{Segment,SegmentProt};

#[derive(Debug)]
#[allow(dead_code)]	// Ignore non-use of the variant members
pub enum Error
{
	NotElf,
	Unsupported,
	Malformed,
	UndefinedSymbol,
	Vfs(VfsError),
	Byteorder(::byteorder::Error),
	Io(::std::io::Error),
}
impl_from! {
	From<VfsError>(e) for Error {
		Error::Vfs(e)
	}
	From<::byteorder::Error>(e) for Error {
		Error::Byteorder(e)
	}
	From<::std::io::Error>(e) for Error {
		Error::Io(e)
	}
}

pub struct ElfModuleHandle<R: Read+Seek>
{
	file: R,
	header: Header,
}

pub fn load_executable(mut fh: File) -> Result<ElfModuleHandle<File>,Error>
{
	// 1. Open file
	let elf_ident = {
		let mut hdr: [u8; 16] = [0; 16];
		if fh.read(&mut hdr)? != 16 {
			return Err(Error::NotElf);
		}
		hdr
		};
	if elf_ident[0..4] != b"\x7FELF"[..] {
		return Err( Error::NotElf );
	}
	
	// 2. Read header
	::syscalls::log_write("Reading header");
	let hdr = Header::parse_partial(&elf_ident, &mut fh)?;
	::syscalls::log_write("Header read");
	kernel_log!("Text only");
	kernel_log!("elf_ident = {:?}", elf_ident);
	kernel_log!("hdr = {:?}", hdr);
	Ok(ElfModuleHandle{
		file: fh,
		header: hdr,
		})
}
	
impl<R: Read+Seek> ElfModuleHandle<R>
{
	fn phents(&mut self) -> PhEntIterator<'_, R> {
		self.file.seek(SeekFrom::Start(self.header.e_phoff) ).expect("Unable to seek to phoff");
		PhEntIterator {
			file: &mut self.file,
			object_size: self.header.object_size,
			remaining_ents: self.header.e_phnum,
			entry_size: self.header.e_phentsize,
			}
	}
	fn dyntab(&mut self, ofs: u64, len: usize) -> DtEntIterator<'_, R> {
		self.file.seek(SeekFrom::Start(ofs)).expect("Unable to seek to dynamic table offset");
		DtEntIterator {
			file: &mut self.file,
			size: self.header.object_size,
			remaining_ents: match self.header.object_size {
				Size::Elf32 => len / (2*4),
				Size::Elf64 => len / (2*8),
				},
			}
	}
}

// TODO: Make this part of a trait
impl<R: Read+Seek> ElfModuleHandle<R>
{
	pub fn get_entrypoint(&self) -> usize {
		self.header.e_entry
	}
	pub fn load_segments(&mut self) -> LoadSegments<'_, R> {
		LoadSegments( self.phents() )
	}
	
	pub fn do_relocation(&mut self) -> Result<(),Error> {
		// 1. Locate the PT_DYN section
		let pt_dyn = match self.phents().find(|e| e.p_type == PT_DYNAMIC)
			{
			Some(e) => e,
			None => return Ok( () ),	// No PT_DYN, nothing to do
			};
		kernel_log!("pt_dyn = {:?}", pt_dyn);
		// 2. Parse to locate the symbol table, string table, and Rel/Rela sections
		let (mut symtab_addr,mut symtab_esz) = (None, None);
		let (mut strtab_addr,mut strtab_len) = (None, None);
		let (mut rel_addr, mut rel_sz, mut rel_esz) = Default::default();
		let (mut rela_addr, mut rela_sz, mut rela_esz) = Default::default();
		let (mut plt_addr, mut plt_sz, mut plt_type) = (None, None, RelocType::RelA);
		for ent in self.dyntab(pt_dyn.p_offset, pt_dyn.p_filesz)
		{
			match ent
			{
			DtEnt::SymTab(addr) => symtab_addr = Some(addr as *const _),
			DtEnt::SymEntSz(count) => symtab_esz = Some(count),
			DtEnt::StrTab(addr) => strtab_addr = Some(addr as *const _),
			DtEnt::StrSz(count) => strtab_len = Some(count),
			
			DtEnt::RelA(addr) => rela_addr = Some(addr),
			DtEnt::RelASz(size) => rela_sz = Some(size),
			DtEnt::RelAEnt(size) => rela_esz = Some(size),
			
			DtEnt::Rel(addr) => rel_addr = Some(addr),
			DtEnt::RelSz(size) => rel_sz = Some(size),
			DtEnt::RelEnt(size) => rel_esz = Some(size),
			
			DtEnt::Plt(addr) => plt_addr = Some(addr),
			DtEnt::PltRel(ty) => plt_type = match ty {
				 7 => RelocType::RelA,	// DT_RELA
				17 => RelocType::Rel,	// DT_REL
				_ => {
					kernel_log!("Malformed ELF - PltRel type invalid {}", ty);
					return Err(Error::Malformed)
					},
				},
			DtEnt::PltRelSz(size) => plt_sz = Some(size),
			DtEnt::Needed(_) => {/* do nothing */},
			//v @ _ => kernel_log!("- ?{:?}", v),
			_ => {},
			}
		}
		kernel_log!("symtab_ofs = {:?}, strtab_ofs = {:?}", symtab_addr, strtab_addr);
		// SAFE: (well, as can be) These addresses should be pointing to within the program's image
		let (strtab, symtab, rel, rela, plt) = unsafe {
			let strtab = StringTable::new(strtab_addr,strtab_len)?;
			// TODO: Check assumption that symtab_addr < strtab_addr
			let symtab = SymbolTable::new(self.header.get_format(), symtab_addr, strtab_addr.map(|x| x as usize - symtab_addr.unwrap_or(x as *const _) as usize), symtab_esz)?;
			let rel  = RelocTable::new(self.header.get_format(), rel_addr , rel_sz , rel_esz , RelocType::Rel )?;
			let rela = RelocTable::new(self.header.get_format(), rela_addr, rela_sz, rela_esz, RelocType::RelA)?;
			let plt  = RelocTable::new(self.header.get_format(), plt_addr, plt_sz, None, plt_type)?;
			(strtab, symtab, rel, rela, plt)
			};
		
		kernel_log!("strtab = {:?}", ::std::ffi::OsStr::new(strtab.0));
		// Have symbol table - Nice, can't relocate it due to the way it's yielded though
		// - Should really get a structure that allows random access to it, for resolution
		for sym in symtab.iter()
		{
			kernel_log!("- {:?}", sym);
		}
		
		// 1. Locate DT_NEEDED entries and load the relevant libraries
		for ent in self.dyntab(pt_dyn.p_offset, pt_dyn.p_filesz)
		{
			if let DtEnt::Needed(ofs) = ent {
				if let Some(name) = strtab.get(ofs) {
					kernel_log!("DT_NEEDED '{:?}'", name);
					if name != "libloader_dyn.so" {
						//::load::load_library(name);
						panic!("TODO: Load library '{:?}'", name);
					}
				}
				else {
				}
			}
		}
		
		// 2. Iterate the Rel/RelA/PLT relocation lists and apply
		kernel_log!("Relocations:");
		for r in rel.iter() {
			kernel_log!("REL {:?}", r);
		}
		for r in rela.iter() {
			kernel_log!("RELA {:?}", r);
		}
		for r in plt.iter() {
			kernel_log!("PLT {:?}", r);
		}
		
		kernel_log!("Applying relocations:");
		{
			let rs = RelocationState {
				base: 0,
				format: self.header.get_format(),
				machine: self.header.machine,
				strtab: strtab,
				symtab: symtab,
				};
			rs.apply_relocs( rel.iter().chain(rela.iter()).chain(plt.iter()) )?;
		}
		
		Ok( () )
	}
}

struct RelocationState<'a>
{
	base: usize,
	format: Format,
	machine: Machine,
	symtab: SymbolTable<'a>,
	strtab: StringTable<'a>,
}

impl<'a> RelocationState<'a>
{
	fn apply_relocs<I: Iterator<Item=Reloc>>(&self, iter: I) -> Result<(), Error>
	{
		match self.machine
		{
		Machine::I386 => todo!("apply_relocs - Machine {:?}", self.machine),
		Machine::X8664 => for r in iter { self.apply_reloc_x86_64(r)?; },
		Machine::ARM => for r in iter { self.apply_reloc_arm(r)?; },
		Machine::Riscv => for r in iter { self.apply_reloc_riscv(r)?; },
		Machine::Aarch64 => for r in iter { self.apply_reloc_aarch64(r)?; },
		_ => todo!("apply_relocs - Machine {:?}", self.machine),
		}
		Ok( () )
	}
	fn apply_reloc_arm(&self, r: Reloc) -> Result<(), Error> {
		const R_ARM_NONE: u16 = 0;
		//const R_ARM_PC24: u16 = 1;	// ((S + A) | T) - P
		const R_ARM_JUMP_SLOT: u16 = 22;	// (S + A) | T
		match r.ty
		{
		R_ARM_NONE => {},
		R_ARM_JUMP_SLOT => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_32(r.addr, |_val| addr as u32);
			},
		v @ _ => todo!("apply_reloc_arm - ty={}", v),
		}
		Ok( () )
	}

	fn apply_reloc_x86_64(&self, r: Reloc) -> Result<(), Error> {
		const R_X86_64_NONE : u16 = 0;
		const R_X86_64_64   : u16 = 1;	// 64, S + A
		const R_X86_64_PC32 : u16 = 2; 	// 32, S + A - P
		const R_X86_64_GOT32: u16 = 3;	// 32, G + A
		const R_X86_64_PLT32: u16 = 4;	// 32, L + A - P
		const R_X86_64_COPY : u16 = 5;
		const R_X86_64_GLOB_DAT : u16 = 6;	// 64, S
		const R_X86_64_JUMP_SLOT: u16 = 7;	// 64, S
		const R_X86_64_RELATIVE : u16 = 8;	// 64, B + A

		match r.ty
		{
		R_X86_64_NONE => {},
		R_X86_64_64 => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_64(r.addr, |val| (addr + r.addend.unwrap_or(val as usize)) as u64);
			},
		R_X86_64_PC32 => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_32(r.addr, |val| (addr + r.addend.unwrap_or(val as usize) - r.addr) as u32);
			},
		R_X86_64_GOT32 => todo!("apply_reloc_x86_64 - GOT32"),
		R_X86_64_PLT32 => todo!("apply_reloc_x86_64 - PLT32"),
		R_X86_64_COPY => todo!("apply_reloc_x86_64 - COPY"),
		R_X86_64_GLOB_DAT => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_64(r.addr, |_val| addr as u64);
			},
		R_X86_64_JUMP_SLOT => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_64(r.addr, |_val| addr as u64);
			},
		R_X86_64_RELATIVE => {
			self.relocate_64(r.addr, |val| (self.base + r.addend.unwrap_or(val as usize)) as u64);
			},
		v @ _ => todo!("apply_reloc_x86_64 - ty={}", v),
		}
		Ok( () )
	}

	fn apply_reloc_riscv(&self, r: Reloc) -> Result<(), Error> {
		const R_RISCV_NONE: u16 = 0;
		match r.ty
		{
		R_RISCV_NONE => {},
		1 /*R_RISCV_32*/ =>  {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_32(r.addr, |val| (addr + r.addend.unwrap_or(val as usize)) as u32);
			},
		2 /*R_RISCV_64*/ => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_64(r.addr, |val| (addr + r.addend.unwrap_or(val as usize)) as u64);
			},
		5 /*R_RISCV_JUMP_SLOT*/ => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			match self.format.size
			{
			Size::Elf32 => self.relocate_32(r.addr, |_val| addr as u32),
			Size::Elf64 => self.relocate_64(r.addr, |_val| addr as u64),
			}
			},
		v @ _ => todo!("apply_reloc_riscv64 - ty={}", v),
		}
		Ok( () )
	}
	fn apply_reloc_aarch64(&self, r: Reloc) -> Result<(), Error> {
		match r.ty
		{
		0 => {},
		1024 /* R_AARCH64_COPY */  => todo!("apply_reloc_aarch64 - COPY"),
		1025 /* R_AARCH64_GLOB_DAT */ => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_64(r.addr, |_val| addr as u64);
			},
		1026 /* R_AARCH64_JUMP_SLOT */ => {
			let (addr,_size) = self.get_symbol_r(r.sym as usize)?;
			self.relocate_64(r.addr, |_val| addr as u64);
			},
		v @ _ => todo!("apply_reloc_aarch64 - ty={}", v),
		}
		Ok( () )
	}
	
	
	fn get_symbol(&self, idx: usize) -> Option<(usize, usize)> {
		if let Some(sym) = self.symtab.get(idx)
		{
			let name = match self.strtab.get(sym.st_name)
				{
				Some(v) => v,
				None => { kernel_log!("Malformed ELF, symbol {} name {} invalid", idx, sym.st_name); return None; },
				};
			kernel_log!("get_symbol: #{} = {:?} {:?}", idx, sym, name);
			if sym.st_shndx == 0 {
				::load::lookup_symbol(name)
			}
			else {
				Some( (self.base + sym.st_value, sym.st_size) )
			}
		}
		else {
			// TODO: This is kinda... not correct, as it actually means a malformed file
			None
		}
	}
	fn get_symbol_r(&self, idx: usize) -> Result<(usize, usize), Error> {
		match self.get_symbol(idx)
		{
		Some(v) => Ok(v),
		None => Err(Error::UndefinedSymbol),
		}
	}
	
	fn relocate_64<F: FnOnce(u64)->u64>(&self, addr: usize, fcn: F) {
		// SAFE: (uncheckable) Assumes that the file is valid
		unsafe {
			// TODO: Ensure that address is valid (i.e. within the newly loaded sections)
			let ptr = addr as *mut u64;
			// TODO: Ensure that endianness is native endian
			*ptr = fcn(*ptr);
		}
	}
	fn relocate_32<F: FnOnce(u32)->u32>(&self, addr: usize, fcn: F) {
		// SAFE: (uncheckable) Assumes that the file is valid
		unsafe {
			// TODO: Ensure that address is valid
			let ptr = addr as *mut u32;
			// TODO: Ensure that endianness is native endian
			*ptr = fcn(*ptr);
		}
	}
}

struct StringTable<'a>(&'a [u8]);
impl<'a> StringTable<'a>
{
	unsafe fn new<'b>(addr: Option<*const u8>, len: Option<usize>) -> Result<StringTable<'b>,Error> {
		let strtab = match (addr,len) {
			(Some(a), Some(l)) => ::std::slice::from_raw_parts(a, l),
			(None, None) => &[][..],
			_ => {
				kernel_log!("Malformed ELF - String table addr or len passed (addr={:?},len={:?})", addr, len);
				return Err(Error::Malformed)
				},
			};
		Ok( StringTable(strtab) )
	}
	
	fn get(&self, ofs: usize) -> Option<&::std::ffi::OsStr> {
		if ofs >= self.0.len() {
			None
		}
		else {
			Some(::std::ffi::OsStr::new( self.0[ofs..].split(|&x| x==0).next().unwrap() ))
		}
	}
}

#[allow(dead_code)]
struct PHEnt
{
	p_type: u32,
	p_flags: u32,
	p_offset: u64,
	p_vaddr: usize,
	p_paddr: usize,	// aka load
	p_filesz: usize,
	p_memsz: usize,
	p_align: usize,	
}
impl_fmt! {
	Debug(self,f) for PHEnt {
		write!(f, "PHEnt {{ p_type: {}, p_flags: {:#x}, p_offset: {:#x}, p_vaddr: {:#x}, p_paddr: {:#x}, p_filesz: {:#x}, p_memsz: {:#x}, p_align: {}",
			self.p_type,
			self.p_flags,
			self.p_offset,
			self.p_vaddr,
			self.p_paddr,
			self.p_filesz,
			self.p_memsz,
			self.p_align)
	}
}
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
impl PHEnt
{
	fn parse_64<R: Read>(file: &mut R) -> Result<PHEnt,Error>
	{
		use byteorder::{ReadBytesExt,LittleEndian};
		Ok(PHEnt {
			p_type:   file.read_u32::<LittleEndian>()?,
			p_flags:  file.read_u32::<LittleEndian>()?,
			p_offset: file.read_u64::<LittleEndian>()?,
			p_vaddr:  file.read_u64::<LittleEndian>()? as usize,
			p_paddr:  file.read_u64::<LittleEndian>()? as usize,
			p_filesz: file.read_u64::<LittleEndian>()? as usize,
			p_memsz:  file.read_u64::<LittleEndian>()? as usize,
			p_align:  file.read_u64::<LittleEndian>()? as usize,
			})
	}
	fn parse_32<R: Read>(file: &mut R) -> Result<PHEnt,Error>
	{
		use byteorder::{ReadBytesExt,LittleEndian};
		Ok(PHEnt {
			p_type:   file.read_u32::<LittleEndian>()?,
			p_offset: file.read_u32::<LittleEndian>()? as u64,
			p_vaddr:  file.read_u32::<LittleEndian>()? as usize,
			p_paddr:  file.read_u32::<LittleEndian>()? as usize,
			p_filesz: file.read_u32::<LittleEndian>()? as usize,
			p_memsz:  file.read_u32::<LittleEndian>()? as usize,
			p_flags:  file.read_u32::<LittleEndian>()?,
			p_align:  file.read_u32::<LittleEndian>()? as usize,
			})
	}
}
struct PhEntIterator<'a, R: 'a + Read>
{
	file: &'a mut R,	// File is pre-seeked to the start of the PHENT list
	object_size: Size,
	remaining_ents: u16,
	entry_size: u16,
}
impl<'a, R: 'a+Read>  PhEntIterator<'a, R> {
	fn read_entry(&mut self) -> Result<PHEnt, Error> {
		let mut data = [0; 64];
		assert!(self.entry_size as usize <= data.len(), "Allocation {} insufficient for {}", data.len(), self.entry_size);
		let data = &mut data[.. self.entry_size as usize];
		if self.file.read(data)? != self.entry_size as usize {
			panic!("TODO");
		}
		else {
			match self.object_size
			{
			Size::Elf64 => PHEnt::parse_64(&mut &*data),
			Size::Elf32 => PHEnt::parse_32(&mut &*data),
			}
		}
	}
}
impl<'a, R: 'a+Read> ::std::iter::Iterator for PhEntIterator<'a, R> {
	type Item = PHEnt;
	
	fn next(&mut self) -> Option<PHEnt> {
		if self.remaining_ents == 0 {
			None
		}
		else {
			self.remaining_ents -= 1;
			Some( self.read_entry().expect("Error reading ELF PHEnt") )
		}
	}
}

#[derive(Debug)]
enum DtEnt {
	Null,
	Needed(usize),
	Plt(*const u8), PltRelSz(usize),
	PltRel(usize),
	#[allow(dead_code)]
	PltGot(*const u8),
	#[allow(dead_code)]
	Hash(usize),
	StrTab(*const u8),
	SymTab(*const Symbol),
	RelA(*const u8), RelASz(usize), RelAEnt(usize),
	StrSz(usize),
	SymEntSz(usize),
	Rel(*const u8), RelSz(usize), RelEnt(usize),
	Unknown(#[allow(dead_code)] u8, #[allow(dead_code)] u64),
}
impl_from! {
	From<[u32; 2]>(v) for DtEnt {
		DtEnt::from([ v[0] as u64, v[1] as u64 ])
	}
	From<[u64; 2]>(v) for DtEnt {{
		let val = v[1] as usize;
		match v[0]
		{
		0 => DtEnt::Null,
		1 => DtEnt::Needed(val),
		2 => DtEnt::PltRelSz(val),
		3 => DtEnt::PltGot(val as *const _),
		4 => DtEnt::Hash(val),
		5 => DtEnt::StrTab(val as *const _),
		6 => DtEnt::SymTab(val as *const _),
		7 => DtEnt::RelA(val as *const _),
		8 => DtEnt::RelASz(val),
		9 => DtEnt::RelAEnt(val),
		10 => DtEnt::StrSz(val),
		11 => DtEnt::SymEntSz(val),
		//12 = DT_INIT
		//13 = DT_FINI
		//14 = DT_SONAME
		//15 = DT_RPATH
		//16 = DT_SYMBOLIC
		17 => DtEnt::Rel(val as *const _),
		18 => DtEnt::RelSz(val),
		19 => DtEnt::RelEnt(val),
		20 => DtEnt::PltRel(val),
		//21 = DT_DEBUG
		//22 = DT_TEXTREL
		23 => DtEnt::Plt(val as *const _),
		t @ _ => DtEnt::Unknown(t as u8, v[1]),
		}
	}}
}
struct DtEntIterator<'a, R: 'a+Read>
{
	file: &'a mut R,
	size: Size,
	remaining_ents: usize,
}
impl<'a, R: 'a+Read> DtEntIterator<'a, R> {
	fn get_words(&mut self) -> Result<[u64; 2],Error> {
		use byteorder::{ReadBytesExt,LittleEndian};
		Ok(match self.size
		{
		Size::Elf32 => {
			let mut d = [0; 4*2];
			if self.file.read(&mut d)? != d.len() {
				return Err( Error::Malformed );
			}
			else {
				let mut p = &d[..];
				[ p.read_u32::<LittleEndian>()? as u64, p.read_u32::<LittleEndian>()? as u64 ]
			}
			},
		Size::Elf64 => {
			let mut d = [0; 8*2];
			if self.file.read(&mut d)? != d.len() {
				return Err( Error::Malformed );
			}
			else {
				let mut p = &d[..];
				[ p.read_u64::<LittleEndian>()?, p.read_u64::<LittleEndian>()? ]
			}
			},
		})
	}
}
impl<'a, R: 'a+Read> Iterator for DtEntIterator<'a, R>
{
	type Item = DtEnt;
	fn next(&mut self) -> Option<DtEnt> {
		if self.remaining_ents == 0 {
			None
		}
		else {
			self.remaining_ents -= 1;
			let words = self.get_words().expect("Unexpected error reading dynamic table");
			let ent = DtEnt::from(words);
			if let DtEnt::Null = ent {
				None
			}
			else {
				Some(ent)
			}
		}
	}
}

pub struct LoadSegments<'a, R: 'a + Read>(PhEntIterator<'a,R>);
impl<'a, R: 'a + Read> ::load::SegmentIterator<R> for LoadSegments<'a, R>
{
	fn get_file(&self) -> &R { self.0.file }
}

impl<'a, R: 'a+Read> ::std::iter::Iterator for LoadSegments<'a, R> {
	type Item = Segment;
	fn next(&mut self) -> Option<Self::Item> {
		while let Some(e) = self.0.next()
		{
			if e.p_type == PT_LOAD
			{
				return Some(Segment {
					load_addr: e.p_paddr,
					file_addr: e.p_offset,
					file_size: e.p_filesz,
					mem_size: e.p_memsz,
					protection: match e.p_flags & 7
						{
						0 => continue,
						0x4 => SegmentProt::ReadOnly,
						0x5 => SegmentProt::Execute,
						0x6 => SegmentProt::ReadWrite,
						v @ _ => panic!("TODO: Unknown ELF segment flags {}", v),
						},
					})
			}
		}
		None
	}
}

#[derive(Debug)]
#[allow(dead_code)]
struct Symbol
{
	st_name: usize,	// Offset in string table
	st_value: usize,
	st_size: usize,
	st_info: u8,
	st_other: u8,
	st_shndx: u16,
}

struct SymbolTable<'a>(&'a [u8], Format);
impl<'a> SymbolTable<'a>
{
	unsafe fn new<'b>(fmt: Format, addr: Option<*const Symbol>, len: Option<usize>, esz: Option<usize>) -> Result<SymbolTable<'b>,Error> {
		let bytes = match (addr,len) {
			(Some(a), Some(l)) => {
				if let Some(esz) = esz {
					if esz != Self::ent_size_st(fmt.size) {
						kernel_log!("Malformed Entry - Symbol table entry size invalid - {} != exp {}", esz, Self::ent_size_st(fmt.size));
						return Err(Error::Malformed);
					}
				}
				kernel_log!("SymbolTable::new(addr={:?}, len={})", a, l);
				if l % Self::ent_size_st(fmt.size) != 0 {
					kernel_log!("Malformed Entry - Symbol table entry size invalid - len % {} ({}) != 0",
						Self::ent_size_st(fmt.size),
						l % Self::ent_size_st(fmt.size)
						);
					return Err(Error::Malformed);
				}
				::std::slice::from_raw_parts(a as *const u8, l)
				},
			(None, None) => &[][..],
			_ => return Err(Error::Malformed),
			};
		Ok( SymbolTable(bytes, fmt) )
	}
	
	fn iter(&self) -> SymbolIterator<'_> {
		SymbolIterator {
			tab: self,
			idx: 0,
		}
	}
	fn ent_size_st(sz: Size) -> usize {
		match sz {
		Size::Elf32 => 4*4,
		Size::Elf64 => 3*8,
		}
	}
	fn ent_size(&self) -> usize { Self::ent_size_st(self.1.size) }
	fn len(&self) -> usize {
		self.0.len() / self.ent_size()
	}
	
	fn read_sym32(&self, mut slice: &[u8]) -> Result<Symbol,Error> {
		use byteorder::{ReadBytesExt,LittleEndian};
		Ok(Symbol {
			st_name:  slice.read_u32::<LittleEndian>()? as usize,
			st_value: slice.read_u32::<LittleEndian>()? as usize,
			st_size:  slice.read_u32::<LittleEndian>()? as usize,
			st_info:  slice.read_u8()?,
			st_other: slice.read_u8()?,
			st_shndx: slice.read_u16::<LittleEndian>()?,
			})
		
	}
	fn read_sym64(&self, mut slice: &[u8]) -> Result<Symbol,Error> {
		use byteorder::{ReadBytesExt,LittleEndian};
		Ok(Symbol {
			st_name:  slice.read_u32::<LittleEndian>()? as usize,
			st_info:  slice.read_u8()?,
			st_other: slice.read_u8()?,
			st_shndx: slice.read_u16::<LittleEndian>()?,
			st_value: slice.read_u64::<LittleEndian>()? as usize,
			st_size:  slice.read_u64::<LittleEndian>()? as usize,
			})
	}
	fn get(&self, idx: usize) -> Option<Symbol> {
		if idx >= self.len() {
			None
		}
		else {
			let slice = &self.0[idx*self.ent_size()..];
			let res_sym = match self.1.size
				{
				Size::Elf32 => self.read_sym32(slice),
				Size::Elf64 => self.read_sym64(slice),
				};
			Some(res_sym.unwrap())
		}
	}
}
struct SymbolIterator<'a>
{
	tab: &'a SymbolTable<'a>,
	idx: usize,
}
impl<'a> Iterator for SymbolIterator<'a>
{
	type Item = Symbol;
	fn next(&mut self) -> Option<Self::Item>
	{
		let ret = self.tab.get(self.idx);
		self.idx += 1;
		ret
	}
}

#[derive(Copy,Clone,PartialEq,Debug)]
enum RelocType {
	Rel,
	RelA,
}
#[derive(Debug)]
struct Reloc {
	addr: usize,
	ty: u16,
	sym: u32,
	addend: Option<usize>,
}
impl Reloc {
	fn new_rel(ofs: usize, ty: u32, sym: u32) -> Reloc {
		Reloc {
			addr: ofs,
			ty: ty as u16,
			sym: sym,
			addend: None,
		}
	}
	fn new_rela(ofs: usize, ty: u32, sym: u32, addend: usize) -> Reloc {
		Reloc {
			addr: ofs,
			ty: ty as u16,
			sym: sym,
			addend: Some(addend),
		}
	}
}
struct RelocTable<'a> {
	data: &'a [u8],
	format: Format,
	ty: RelocType,
}
impl<'a> RelocTable<'a> {
	unsafe fn new<'b>(format: Format, addr: Option<*const u8>, size: Option<usize>, entsz: Option<usize>, ty: RelocType) -> Result<RelocTable<'b>,Error> {
		match (addr, size)
		{
		(Some(addr), Some(size)) => {
			if let Some(esz) = entsz {
				if esz != Self::ent_sz(format.size, ty) {
					kernel_log!("Malformed Entry - Relocation table entry size invalid - {} != exp {}", esz, Self::ent_sz(format.size, ty));
					return Err(Error::Malformed);
				}
			}
			let slice = ::std::slice::from_raw_parts(addr, size);
			kernel_log!("RelocTable {{ {:p}+{}, {:?} }}", slice.as_ptr(), slice.len(), ty);
			Ok(RelocTable {
				data: slice,
				format: format,
				ty: ty,
				})
			},
		(None, None) => Ok(RelocTable { data: &[][..], format: format, ty: ty }),
		(_, _) => Err(Error::Malformed),
		}
	}
	fn ent_sz(size: Size, ty: RelocType) -> usize {
		match (size, ty)
		{
		(Size::Elf32, RelocType::Rel ) => 2*4,
		(Size::Elf32, RelocType::RelA) => 3*4,
		(Size::Elf64, RelocType::Rel ) => 2*8,
		(Size::Elf64, RelocType::RelA) => 3*8,
		}
	}
	fn get_ent_sz(&self) -> usize {
		Self::ent_sz(self.format.size, self.ty)
	}
	fn read_rel64(&self, idx: usize) -> Option<Reloc> {
		assert_eq!(self.format.size, Size::Elf64);
		assert_eq!(self.ty, RelocType::Rel);
		let esz = self.get_ent_sz();
		let ofs = idx * esz;
		if ofs + esz > self.data.len() {
			None
		}
		else {
			let mut data = &self.data[idx*esz..];
			let r_offset = self.format.read_u64(&mut data) as usize;
			let r_info   = self.format.read_u64(&mut data);
			Some(Reloc::new_rel(r_offset, (r_info & 0xFFFF_FFFF) as u32, (r_info >> 32) as u32))
		}
	}
	fn read_rela64(&self, idx: usize) -> Option<Reloc> {
		assert_eq!(self.format.size, Size::Elf64);
		assert_eq!(self.ty, RelocType::RelA);
		let esz = self.get_ent_sz();
		let ofs = idx * esz;
		if ofs + esz > self.data.len() {
			None
		}
		else {
			let mut data = &self.data[idx*esz..];
			let r_offset = self.format.read_u64(&mut data) as usize;
			let r_info   = self.format.read_u64(&mut data);
			let r_addend = self.format.read_u64(&mut data) as usize;
			Some(Reloc::new_rela(r_offset, (r_info & 0xFFFF_FFFF) as u32, (r_info >> 32) as u32, r_addend))
		}
	}
	fn read_rel32(&self, idx: usize) -> Option<Reloc> {
		assert_eq!(self.format.size, Size::Elf32);
		assert_eq!(self.ty, RelocType::Rel);
		let esz = self.get_ent_sz();
		let ofs = idx * esz;
		kernel_log!("read_rel32 - self.data = {:p}+{}, ofs={}", self.data.as_ptr(), self.data.len(), ofs);
		if idx >= self.data.len() / esz {
			None
		}
		else if ofs + esz > self.data.len() {
			None
		}
		else {
			let mut data = &self.data[idx*esz..];
			let r_offset = self.format.read_u32(&mut data) as usize;
			let r_info   = self.format.read_u32(&mut data);
			Some(Reloc::new_rel(r_offset, r_info & 0xFF, r_info >> 8))
		}
	}
	fn read_rela32(&self, idx: usize) -> Option<Reloc> {
		assert_eq!(self.format.size, Size::Elf32);
		assert_eq!(self.ty, RelocType::RelA);
		let esz = self.get_ent_sz();
		let ofs = idx * esz;
		kernel_log!("read_rela32 - self.data = {:p}+{}, ofs={}", self.data.as_ptr(), self.data.len(), ofs);
		if ofs + esz > self.data.len() {
			None
		}
		else {
			let mut data = &self.data[idx*esz..];
			let r_offset = self.format.read_u32(&mut data) as usize;
			let r_info   = self.format.read_u32(&mut data);
			let r_addend = self.format.read_u32(&mut data) as usize;
			Some(Reloc::new_rela(r_offset, r_info & 0xFF, r_info >> 8, r_addend))
		}
	}
	
	fn read(&self, idx: usize) -> Option<Reloc> {
		match (self.format.size, self.ty)
		{
		(Size::Elf32, RelocType::Rel ) => self.read_rel32(idx),
		(Size::Elf32, RelocType::RelA) => self.read_rela32(idx),
		(Size::Elf64, RelocType::Rel ) => self.read_rel64(idx),
		(Size::Elf64, RelocType::RelA) => self.read_rela64(idx),
		}
	}
	fn iter(&self) -> RelocIter<'_> {
		RelocIter { ptr: self, idx: 0 }
	}
}
struct RelocIter<'a> {
	ptr: &'a RelocTable<'a>,
	idx: usize,
}
impl<'a> Iterator for RelocIter<'a> {
	type Item = Reloc;
	fn next(&mut self) -> Option<Reloc> {
		let ret = self.ptr.read(self.idx);
		self.idx += 1;
		ret
	}
}


#[derive(Copy,Clone,PartialEq,Debug)]
enum Size { Elf32, Elf64 }
#[derive(Copy,Clone,PartialEq,Debug)]
enum Endian { Little, Big }
#[derive(Copy,Clone,Debug)]
enum ObjectType { None, Reloc, Exec, Dyn, Core, Unk(#[allow(dead_code)] u16) }
#[derive(Copy,Clone,Debug)]
enum Machine {
	None,
	I386,
	ARM,
	X8664,
	Aarch64,
	Riscv,
	Unk(#[allow(dead_code)] u16)
}
impl_from! {
	From<u16>(v) for ObjectType {
		match v
		{
		0 => ObjectType::None,
		1 => ObjectType::Reloc,
		2 => ObjectType::Exec,
		3 => ObjectType::Dyn,
		4 => ObjectType::Core,
		_ => ObjectType::Unk(v),
		}
	}
	From<u16>(v) for Machine {
		match v
		{
		0 => Machine::None,
		3 => Machine::I386,
		40 => Machine::ARM,
		62 => Machine::X8664,
		183 => Machine::Aarch64,
		243 => Machine::Riscv,
		_ => Machine::Unk(v),
		}
	}
}

struct Format
{
	size: Size,
	endian: Endian,
}
impl Format
{
	fn read_u64(&self, buf: &mut dyn Read) -> u64 {
		use byteorder::{ReadBytesExt,LittleEndian,BigEndian};
		match self.endian {
		Endian::Little => buf.read_u64::<LittleEndian>().unwrap(),
		Endian::Big    => buf.read_u64::<BigEndian>().unwrap(),
		}
	}
	fn read_u32(&self, buf: &mut dyn Read) -> u32 {
		use byteorder::{ReadBytesExt,LittleEndian,BigEndian};
		match self.endian {
		Endian::Little => buf.read_u32::<LittleEndian>().unwrap(),
		Endian::Big    => buf.read_u32::<BigEndian>().unwrap(),
		}
	}
}

#[derive(Debug)]
#[allow(dead_code)]
struct Header
{
	object_size: Size,
	endian: Endian,
	object_type: ObjectType,
	machine: Machine,
	
	e_entry: usize,
	e_phoff: u64,
	e_phentsize: u16,
	e_phnum: u16,
}

impl Header
{
	fn parse_partial<R: ::std::io::Read>(ident: &[u8], file: &mut R) -> Result<Header, Error> {
		use byteorder::{ReadBytesExt,LittleEndian};
		let objsize = match ident[4]
			{
			1 => Size::Elf32,
			2 => Size::Elf64,
			_ => return Err(Error::Unsupported),
			};
		let endian = match ident[5]
			{
			1 => Endian::Little,
			2 => Endian::Big,
			_ => return Err(Error::Unsupported),
			};
		assert_eq!(endian, Endian::Little);
		
		match objsize
		{
		Size::Elf32 => {
			let data = { let mut d = [0; 36]; file.read(&mut d)?; d };
			let mut data = &data[..];

			let objtype = ObjectType::from( data.read_u16::<LittleEndian>()? );
			let machine = Machine::from( data.read_u16::<LittleEndian>()? );
			let version = data.read_u32::<LittleEndian>()?;
			if version != 1 {
				kernel_log!("Unknown elf version: {}", version);
				return Err(Error::Unsupported);
			}
			let e_entry = data.read_u32::<LittleEndian>()?;
			let e_phoff = data.read_u32::<LittleEndian>()?;
			let _e_shoff = data.read_u32::<LittleEndian>()?;
			let _e_flags = data.read_u32::<LittleEndian>()?;
			let _e_ehsize = data.read_u16::<LittleEndian>()?;
			let e_phentsize = data.read_u16::<LittleEndian>()?;
			let e_phnum     = data.read_u16::<LittleEndian>()?;
			let _e_shentsize = data.read_u16::<LittleEndian>()?;
			let _e_shnum     = data.read_u16::<LittleEndian>()?;
			let _e_shstrndx  = data.read_u16::<LittleEndian>()?;
			Ok( Header {
				object_size: Size::Elf32, endian: endian,
				object_type: objtype,
				machine: machine,
				
				e_entry: e_entry as usize,
				e_phoff: e_phoff as u64,
				e_phentsize: e_phentsize,
				e_phnum: e_phnum,
				})
			},
		Size::Elf64 => {
			let data = { let mut d = [0; 48]; file.read(&mut d)?; d };
			let mut data = &data[..];

			let objtype = ObjectType::from( data.read_u16::<LittleEndian>()? );
			let machine = Machine::from( data.read_u16::<LittleEndian>()? );
			let version = data.read_u32::<LittleEndian>()?;
			if version != 1 {
				kernel_log!("Unknown elf version: {}", version);
				return Err(Error::Unsupported);
			}
			let e_entry = data.read_u64::<LittleEndian>()?;
			let e_phoff = data.read_u64::<LittleEndian>()?;
			let _e_shoff = data.read_u64::<LittleEndian>()?;
			let _e_flags = data.read_u32::<LittleEndian>()?;
			let _e_ehsize = data.read_u16::<LittleEndian>()?;
			let e_phentsize = data.read_u16::<LittleEndian>()?;
			let e_phnum     = data.read_u16::<LittleEndian>()?;
			let _e_shentsize = data.read_u16::<LittleEndian>()?;
			let _e_shnum     = data.read_u16::<LittleEndian>()?;
			let _e_shstrndx  = data.read_u16::<LittleEndian>()?;
			Ok( Header {
				object_size: Size::Elf64, endian: endian,
				object_type: objtype,
				machine: machine,
				
				e_entry: e_entry as usize,
				e_phoff: e_phoff,
				e_phentsize: e_phentsize,
				e_phnum: e_phnum,
				})
			},
		}
	}
	
	fn get_format(&self) -> Format {
		Format {
			size: self.object_size,
			endian: self.endian,
		}
	}
}

