use tifflin_syscalls::vfs::{File,FileOpenMode};
use tifflin_syscalls::vfs::Error as VfsError;

use std::io::{Read,Seek};

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
	
	let hdr = try!( Header::parse_partial(&elf_ident, &mut fh) );
	kernel_log!("hdr = {:?}", hdr);
	// 2. Read header
	unimplemented!();
}

impl<R: Read+Seek> ElfModuleHandle<R>
{
	pub fn get_entrypoint(&self) -> fn(&[&str])->! {
		unimplemented!();
	}
	pub fn load_segments(&self) -> LoadSegments<R> {
		unimplemented!();
	}
}
pub struct LoadSegments<'a, R: 'a + Read+Seek>(&'a mut R, u32);
impl<'a, R: 'a+Read+Seek> ::std::iter::Iterator for LoadSegments<'a, R> {
	type Item = ();
	fn next(&mut self) -> Option<Self::Item> {
		unimplemented!()
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

