
use ::std::io::{Write,Read};

fn main() {
	let mut a = ::std::env::args_os();
	a.next().unwrap();
	let dst = a.next().expect("Usage: <dst> <src>");
	let src = a.next().expect("Usage: <dst> <src>");

	let mut src = ::std::fs::File::open(src).expect("Unable to open source");
	let mut dst = ::std::fs::File::create_new(dst).expect("Unable to open destination");
	let len = {
		use ::std::io::{Seek,SeekFrom};
		let len = src.seek(SeekFrom::End(0)).expect("Unable to find end of file");
		src.seek(SeekFrom::Start(0)).unwrap();
		len
	};

	//let chunk_size = 16*1024;	// 16K chunks should seek quite quickly
	let chunk_size = 1<<16;

	dst.write(&::compressed_file::Header {
		magic: ::compressed_file::MAGIC,
		block_size: chunk_size as u32,
		file_size: len,
	}.to_bytes()).expect("Unable to write header");

	fn tell(v: &mut ::std::fs::File) -> u64 {
		::std::io::Seek::seek(v, ::std::io::SeekFrom::Current(0)).unwrap()
	}

	let mut buf = vec![0; chunk_size];
	loop {
		let pos = tell(&mut src);
		let len = src.read(&mut buf).expect("Failed to read");
		if len == 0 {
			break;
		}
		if pos < 1<<20 {
			println!("CHUNK {:#x} at {:#x}", pos, tell(&mut dst));
		}

		let mut dst_buf = vec![];

		{
			let mut w = ::flate2::write::ZlibEncoder::new(&mut dst_buf, ::flate2::Compression::best());
			w.write_all(&buf).expect("Failed to write compressed data");
			w.finish().expect("Failed to finalise compressed data");
		}

		dst.write_all(&(dst_buf.len() as u32).to_le_bytes()).expect("Failed to write block length");
		dst.write_all(&dst_buf).expect("Failed to write compressed data");
		if len < buf.len() {
			break;
		}
	}
}