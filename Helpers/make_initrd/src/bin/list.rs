use std::io::Read;

struct Arguments {
	input: ::std::path::PathBuf,
}

fn main() {
	let args = Arguments::parse_from_args();
	let data = {
		let mut data = Vec::new();
		let mut fp = ::std::fs::File::open(&args.input).unwrap();
		fp.read_to_end(&mut data).unwrap();
		data
	};

	let header = unsafe { &*(data.as_ptr() as *const initrd_repr::Header) };
	let inodes = unsafe { ::std::slice::from_raw_parts(
		(data.as_ptr() as *const initrd_repr::Header).offset(1) as *const initrd_repr::Inode,
		header.node_count as usize
	) };

	let indent = 0;
	dump_file(&data, inodes, 0, indent);
}

impl Arguments {
	fn parse_from_args() -> Self {
		let mut input = None;
		for v in ::std::env::args().skip(1) {
			if input.is_none() {
				input = Some(::std::path::Path::new(&v).to_owned());
			}
			else {
				panic!();
			}
		}
		Arguments {
			input: input.unwrap(),
		}
	}
}
fn dump_file(data: &[u8], inodes: &[initrd_repr::Inode], inode_idx: u32, indent: usize) {
	let i = &inodes[inode_idx as usize];
	let d = &data[i.ofs as usize..][..i.length as usize];
	println!("#{} {} @{:#x}+{:#x}", inode_idx, i.ty, i.ofs, i.length);
	if i.ty == initrd_repr::NODE_TY_DIRECTORY {
		use initrd_repr::DirEntry;
		if d.as_ptr() as usize % align_of::<DirEntry>() != 0 {
			return;
		}
		if d.len() as usize % size_of::<DirEntry>() != 0 {
			return;
		}
		// SAFE: Alignment and size checked above, data is functionally POD
		let ents = unsafe { ::core::slice::from_raw_parts(d.as_ptr() as *const DirEntry, d.len() / size_of::<DirEntry>()) };
		for e in ents {
			for _ in 0..indent {
				print!("  ");
			}
			print!("- {:?}: ", ::std::str::from_utf8(trim_nuls(&e.filename)));
			dump_file(data, inodes, e.node, indent+1);
		}
	}
	else {
		let l = d.len().min( 32 );
		let d = &d[..l];
		for _ in 0..indent+1 {
			print!("  ");
		}
		println!("{:x?}", d);
	}
}
fn trim_nuls(name: &[u8]) -> &[u8] {
	let l = name.iter().position(|v| *v == 0).unwrap_or(name.len());
	&name[..l]
}