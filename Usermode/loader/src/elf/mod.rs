use tifflin_syscalls::vfs::{File,FileOpenMode};
use tifflin_syscalls::vfs::Error as VfsError;

use std::io::{Read,Seek,SeekFrom};

#[derive(Debug)]
pub enum Error
{
	NotElf,
	Unsupported,
	Vfs(VfsError),
}
impl_from! {
	From<VfsError>(e) for Error {
		Error::Vfs(e)
	}
	From<::byteorder::Error>(e) for Error {
		panic!("")
	}
	From<::std::io::Error>(e) for Error {
		panic!("")
	}
}

pub struct ElfModuleHandle<R: Read+Seek>
{
	file: R,
	header: Header,
}

pub fn load_executable(path: &str) -> Result<ElfModuleHandle<File>,Error>
{
	// 1. Open file
	let mut fh = try!(File::open(path, FileOpenMode::Execute));
	let elf_ident = {
		let mut hdr: [u8; 16] = [0; 16];
		if try!(fh.read(&mut hdr)) != 16 {
			return Err(Error::NotElf);
		}
		hdr
		};
	if elf_ident[0..4] != b"\x7FELF"[..] {
		return Err( Error::NotElf );
	}
	
	// 2. Read header
	let hdr = try!( Header::parse_partial(&elf_ident, &mut fh) );
	kernel_log!("hdr = {:?}", hdr);
	Ok(ElfModuleHandle{
		file: fh,
		header: hdr,
		})
}

impl<R: Read+Seek> ElfModuleHandle<R>
{
	pub fn get_entrypoint(&self) -> usize {
		self.header.e_entry
	}
	pub fn load_segments(&mut self) -> LoadSegments<R> {
		self.file.seek(SeekFrom::Start(self.header.e_phoff) );
		LoadSegments {
			file: &mut self.file,
			remaining_ents: self.header.e_phnum,
			entry_size: self.header.e_phentsize,
			}
	}
}
pub struct LoadSegments<'a, R: 'a + Read>
{
	file: &'a mut R,	// File is pre-seeked to the start of the PHENT list
	remaining_ents: u16,
	entry_size: u16,
}
pub struct Segment {
	pub load_addr: usize,
	pub file_addr: u64,
	pub file_size: usize,
	pub mem_size: usize,
	pub protection: SegmentProt,
}
impl_fmt! {
	Debug(self, f) for Segment {
		write!(f, "Segment {{ {:#x}+{:#x} <= {:#x}+{:#x} {:?} }}",
			self.load_addr, self.mem_size,
			self.file_size, self.file_size,
			self.protection
			)
	}
}
#[derive(Debug)]
pub enum SegmentProt {
	Execute,
	ReadOnly,
	ReadWrite,
}
impl<'a, R: 'a+Read>  LoadSegments<'a, R> {
	pub fn get_file(&self) -> &R { self.file }
	fn read_entry(&mut self) -> Result<PHEnt, Error> {
		let mut data = [0; 64];
		assert!(self.entry_size as usize <= data.len(), "Allocation {} insufficent for {}", data.len(), self.entry_size);
		let data = &mut data[.. self.entry_size as usize];
		if try!(self.file.read(data)) != self.entry_size as usize {
			panic!("TODO");
		}
		else {
			PHEnt::parse(&mut &*data)
		}
	}
}
impl<'a, R: 'a+Read> ::std::iter::Iterator for LoadSegments<'a, R> {
	type Item = Segment;
	fn next(&mut self) -> Option<Self::Item> {
		while self.remaining_ents > 0
		{
			self.remaining_ents -= 1;
			let e = self.read_entry().expect("Error reading ELF PHEnt");
			if e.p_type == 1
			{
				return Some(Segment {
					load_addr: e.p_paddr,
					file_addr: e.p_offset,
					file_size: e.p_filesz,
					mem_size: e.p_memsz,
					protection: match e.p_flags & 7
						{
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
enum Size { Elf32, Elf64 }
#[derive(PartialEq,Debug)]
enum Endian { Little, Big }
#[derive(Debug)]
enum ObjectType { None, Reloc, Exec, Dyn, Core, Unk(u16) }
#[derive(Debug)]
enum Machine { None, I386, X86_64, Unk(u16) }
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
		62 => Machine::X86_64,
		_ => Machine::Unk(v),
		}
	}
}
#[derive(Debug)]
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
		Size::Elf32 => panic!("TODO: Header::parse_partial"),
		Size::Elf64 => {
			let objtype = ObjectType::from( try!(file.read_u16::<LittleEndian>()) );
			let machine = Machine::from( try!(file.read_u16::<LittleEndian>()) );
			let version = try!(file.read_u32::<LittleEndian>());
			let e_entry = try!(file.read_u64::<LittleEndian>());
			let e_phoff = try!(file.read_u64::<LittleEndian>());
			let _e_shoff = try!(file.read_u64::<LittleEndian>());
			let _e_flags = try!(file.read_u32::<LittleEndian>());
			let _e_ehsize = try!(file.read_u16::<LittleEndian>());
			let e_phentsize = try!(file.read_u16::<LittleEndian>());
			let e_phnum     = try!(file.read_u16::<LittleEndian>());
			let _e_shentsize = try!(file.read_u16::<LittleEndian>());
			let _e_shnum     = try!(file.read_u16::<LittleEndian>());
			let _e_shstrndx  = try!(file.read_u16::<LittleEndian>());
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
}

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
impl PHEnt
{
	fn parse<R: Read>(file: &mut R) -> Result<PHEnt,Error>
	{
		use byteorder::{ReadBytesExt,LittleEndian};
		// TODO: Handle Elf32
		Ok(PHEnt {
			p_type:  try!(file.read_u32::<LittleEndian>()),
			p_flags: try!(file.read_u32::<LittleEndian>()),
			p_offset: try!(file.read_u64::<LittleEndian>()),
			p_vaddr: try!(file.read_u64::<LittleEndian>()) as usize,
			p_paddr: try!(file.read_u64::<LittleEndian>()) as usize,
			p_filesz: try!(file.read_u64::<LittleEndian>()) as usize,
			p_memsz: try!(file.read_u64::<LittleEndian>()) as usize,
			p_align: try!(file.read_u64::<LittleEndian>()) as usize,
			})
	}
}

