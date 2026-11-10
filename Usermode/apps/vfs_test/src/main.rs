// Tifflin OS - VFS Testing Application
// - By John Hodge (thePowersGang)
//
//! Stress-tests the VFS by enumerating all directories and checksumming all files

#[macro_use]
extern crate syscalls;

extern crate crc;

use syscalls::vfs::{Dir,File};
use syscalls::vfs::{NodeType,FileOpenMode};

fn main()
{
	let root: Dir = ::syscalls::threads::S_THIS_PROCESS.receive_object("/").unwrap();

	let mut buffer = [0; 256];
	dump_dir(0, root, &mut buffer);
}

//struct ChainEnt<'a>(&str, Option<&ChainEnt<'a>>);

struct Repeat<T>(usize, T);
impl<T: ::std::fmt::Display> ::std::fmt::Display for Repeat<T> {
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		for _ in 0 .. self.0 {
			self.1.fmt(f)?;
		}
		Ok( () )
	}
}

fn dump_dir(level: usize, handle: Dir, buffer: &mut [u8])
{
	let mut dir_iter = handle.enumerate().unwrap();
	loop
	{
		let node_res = match dir_iter.read_ent(buffer)
			{
			Ok(Some(name)) => {
				if name == b"." || name == b".." {
					continue ;
				}
				kernel_log!("{}-{:?}", Repeat(level, " "), ::std::str::from_utf8(name));
				handle.open_child(name)
				},
			Ok(None) => return,
			Err(_e) => return ,
			};
		
		match node_res
		{
		Ok(node) =>
			match node.class()
			{
			NodeType::File => dump_file(level+1, node.into_file(FileOpenMode::ReadOnly).unwrap()),
			NodeType::Dir => dump_dir(level+1, node.into_dir().unwrap(), buffer),
			_ => {},
			},
		Err(e) => {
			kernel_log!(">> ERROR {:?}", e);
			//return ;
			},
		}
	}
}

// Reads and applies a CRC32 to the file
fn dump_file(level: usize, mut handle: File)
{
	let mut buffer = [0; 8*4096];

	let mut crc = ::crc::Crc32::new();
	loop
	{
		let len = match handle.read(&mut buffer)
			{
			Ok(0) => break,
			Ok(v) => v,
			Err(e) => {
				kernel_log!(">> ERROR {:?}", e);
				return;
				},
			};

		crc.update( &buffer[..len] );
	}
	kernel_log!("{}> CRC32={:#x}", Repeat(level," "), crc.finalise());
}




