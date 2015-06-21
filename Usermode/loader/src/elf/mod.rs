use tifflin_syscalls::vfs::{File,FileOpenMode};
use tifflin_syscalls::vfs::Error as VfsError;

#[derive(Debug)]
pub enum Error
{
	NotElf,
	Vfs(VfsError),
}
impl From<VfsError> for Error {
	fn from(e: VfsError) -> Error { Error::Vfs(e) }
}

enum ElfModuleClass {
	Elf32,
	Elf64,
}
pub struct ElfModuleHandle
{
	class: ElfModuleClass,
	
}

pub fn load_executable(path: &str) -> Result<ElfModuleHandle,Error>
{
	// 1. Open file
	let fh = try!(File::open(path, FileOpenMode::Execute));
	let elf_ident = {
		let mut hdr: [u8; 16] = [0; 16];
		try!(fh.read_at(0, &mut hdr));
		hdr
		};
	if elf_ident[0..4] != b"\x7FELF"[..] {
		return Err( Error::NotElf );
	}
	// 2. Read header
	unimplemented!();
}

impl ElfModuleHandle
{
	pub fn get_entrypoint(&self) -> fn(&[&str]) -> ! {
		unimplemented!();
	}
}

