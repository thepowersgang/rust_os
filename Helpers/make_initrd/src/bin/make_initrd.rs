
use ::std::path::PathBuf;
use ::std::mem::size_of;
use ::initrd_repr as repr;

struct Arguments {
	output_file: PathBuf,
	sources: Vec<(Destination, PathBuf)>,
}

fn main() {
	// Parse arguments
	let args = Arguments::parse_from_args();
	
	// Enumerate required file+directory counts
	let tree = load_file_tree(&args.sources);
	//dbg!(&tree);
	let root_count = tree.len();
	// Flatten ready for writing to the output
	let nodes = flatten_nodes(&tree);
	//dbg!(&nodes);

	// Generate inode table
	let mut data_ofs = size_of::<repr::Header>() + nodes.len() * size_of::<repr::Inode>();
	let pre_data_pad = (128 - data_ofs % 128) % 128;
	data_ofs += pre_data_pad;
	let mut ofp = ::std::io::BufWriter::new( ::std::fs::File::create(&args.output_file).expect("Opening output") );
	write_struct(&mut ofp, repr::Header {
		magic: repr::MAGIC_NUMBER,
		node_count: nodes.len() as u32,
		root_length: (root_count * size_of::<repr::DirEntry>()) as u32,
		total_length: 0,
	});
	let mut offsets = Vec::with_capacity(nodes.len());
	for node in &nodes {
		offsets.push(data_ofs);
		let (len,ty) = match &node.data {
			NodeData::File(src) => (::std::fs::metadata(src).unwrap().len() as usize, repr::NODE_TY_REGULAR),
			NodeData::Dir(v) => (v.buf.len(), repr::NODE_TY_DIRECTORY),
		};
		write_struct(&mut ofp, repr::Inode {
			ofs: data_ofs as u32,
			length: len as u32,
			ty,
			_reserved: [0; 3],
		});
		//println!("#{} @{:#x}+{:#x}", -1, data_ofs, len);
		data_ofs += len;
		data_ofs += (128 - data_ofs % 128) % 128;
	}

	use std::io::Write;
	ofp.write(&[0; 128][..pre_data_pad]).expect("Failed to write pad");
	for (i,(node,exp_ofs)) in Iterator::zip(nodes.iter(), offsets.iter()).enumerate() {
		let _ = i;
		let true_ofs = ::std::io::Seek::seek(&mut ofp, ::std::io::SeekFrom::Current(0)).unwrap() as usize;
		assert!(true_ofs == *exp_ofs);
		//println!("#{} @{:#x} ({:#x})",
		//	i, exp_ofs, true_ofs
		//	);
		let len = match &node.data {
			NodeData::File(src) => {
				let mut src = ::std::fs::File::open(src).unwrap();
				::std::io::copy(&mut src, &mut ofp).unwrap();
				::std::io::Seek::seek(&mut src, ::std::io::SeekFrom::Current(0)).unwrap() as usize
			},
			NodeData::Dir(v) => {
				ofp.write(&v.buf).expect("Failed to write data");
				v.buf.len()
			},
		};
		let pad = (128 - len % 128) % 128;
		ofp.write(&[0; 128][..pad]).expect("Failed to write pad");
	}
}

impl Arguments {
	fn parse_from_args() -> Self {
		let mut dst = None;
		let mut ents = Vec::new();
		for v in ::std::env::args().skip(1) {
			if dst.is_none() {
				dst = Some(::std::path::Path::new(&v).to_owned());
			}
			else {
				let Some((dst,src)) = v.split_once('=') else {
					continue ;
				};
				let src = ::std::path::Path::new(src).to_owned();
				let mut it = dst.split('/');
				if it.next() != Some("") {
					continue ;
				}
				let mut path: Vec<_> = it.map(|v| v.to_owned()).collect();
				let Some(dst) = path.pop() else {
					continue ;
				};
				let dst = if dst == "" {
					Destination::Dir(path)
				}
				else {
					Destination::Named(path, dst.to_owned())
				};
				ents.push((dst, src));
			}
		}
		Arguments {
			output_file: dst.unwrap(),
			sources: ents,
		}
	}
}
enum Destination {
	Named(Vec<String>, String),
	Dir(Vec<String>),
}

type FileTree = ::std::collections::BTreeMap<String,DirEnt>;
#[derive(Debug)]
enum DirEnt {
	Dir(FileTree),
	File(PathBuf)
}
fn get_dir<'a>(mut d: &'a mut FileTree, components: &[String]) -> &'a mut FileTree {
	for ent in components {
		d = match d.entry(ent.clone()).or_insert_with(|| DirEnt::Dir(Default::default()))
			{
			DirEnt::File(_) => panic!(),
			DirEnt::Dir(d) => d,
			};
	}
	d
}
fn load_file_tree(sources: &[(Destination, PathBuf)]) -> FileTree {
	let mut rv = FileTree::default();
	for (dst,src) in sources {
		match dst {
		Destination::Named(dst_path, dst_name) => {
			let dst_dir = get_dir(&mut rv, dst_path);
			// If this is a directory, import the entire dir
			if src.is_dir() {
				let DirEnt::Dir(dst) = dst_dir.entry(dst_name.clone()).or_insert(DirEnt::Dir(Default::default())) else {
					panic!();
				};
				load_dir_files(dst, src)
			}
			else {
				// If it's a single file, just add
				dst_dir.insert(dst_name.clone(), DirEnt::File(src.clone()));
			}
		},
		Destination::Dir(path) => {
			let dst_dir = get_dir(&mut rv, path);
			if src.is_dir() {
				load_dir_files(dst_dir, src);
			}
			else {
				let dst_name = src.file_name().unwrap().to_string_lossy().into_owned();
				dst_dir.insert(dst_name, DirEnt::File(src.clone()));
			}
		},
		}
	}
	rv
}
fn load_dir_files(dst_dir: &mut FileTree, dir: &PathBuf) {
	for ent in ::std::fs::read_dir(dir).unwrap() {
		let ent = ent.unwrap();
		let dst_name = ent.file_name().to_string_lossy().into_owned();
		if dst_name.starts_with(".") {
			continue ;
		}
		let path = ent.path();
		if path.is_dir() {
			let DirEnt::Dir(dst) = dst_dir.entry(dst_name).or_insert(DirEnt::Dir(Default::default())) else {
				panic!();
			};
			load_dir_files(dst, &path);
		}
		else {
			dst_dir.insert(dst_name, DirEnt::File(path));
		}
	}
}
#[derive(Debug)]
struct Node {
	data: NodeData,
}
#[derive(Debug)]
enum NodeData {
	File(PathBuf),
	Dir(DirInner),
}
struct DirInner {
	buf: Vec<u8>,
}
impl ::std::fmt::Debug for DirInner {
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
		for v in self.buf.chunks(size_of::<repr::DirEntry>()) {
			for b in v {
				write!(f, "{:02x} ", b)?;
			}
			f.write_str( if f.alternate() { "\n" } else { "| " })?;
		}
		Ok(())
	}
}
fn flatten_nodes(tree: &FileTree) -> Vec<Node> {
	let mut rv = Vec::new();
	flatten_into(&mut rv, tree);
	rv
}
fn flatten_into(dst: &mut Vec<Node>, tree: &FileTree) {
	let mut entries = Vec::with_capacity( tree.len() * size_of::<repr::DirEntry>() );
	let i = dst.len();
	dst.push(Node {
		data: NodeData::Dir(DirInner { buf: Vec::new() }),
	});
	for (k,v) in tree.iter() {
		write_struct(&mut entries, repr::DirEntry {
			node: dst.len() as u32,
			filename: {
				let mut n = [0; 64-4];
				n[..k.len()].copy_from_slice( k.as_bytes() );
				n
			},
		});

		match v {
		DirEnt::File(src) => {
			dst.push(Node {
				data: NodeData::File(src.clone()),
			});
		},
		DirEnt::Dir(inner) => {
			flatten_into(dst, inner);
		},
		}
	}
	let Node { data: NodeData::Dir(DirInner { buf }) } = &mut dst[i] else { panic!(); };
	*buf = entries;
}

trait ToLe {
	fn to_le_bytes(&self) -> &[u8];
}
impl ToLe for repr::Header {
	fn to_le_bytes(&self) -> &[u8] {
		unsafe { ::std::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) }
	}
}
impl ToLe for repr::Inode {
	fn to_le_bytes(&self) -> &[u8] {
		unsafe { ::std::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) }
	}
}
impl ToLe for repr::DirEntry {
	fn to_le_bytes(&self) -> &[u8] {
		unsafe { ::std::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Self>()) }
	}
}
fn write_struct(dst: &mut impl ::std::io::Write, v: impl ToLe) {
	dst.write(v.to_le_bytes()).expect("write failed");
}