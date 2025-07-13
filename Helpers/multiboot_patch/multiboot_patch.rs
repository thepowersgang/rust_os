
fn main() {
	use std::io::{Read,Write,Seek};
	let args = Args::from_args();
	
	let mut fp = ::std::fs::File::options()
		.read(true)
		.write(true)
		.open(&args.infile)
		.expect("Cannot open")
		;

	// Look for the multiboot signature
	// - Look for 0x1BADB002, aligned to a u32 (4-bytes) in the first 8K
	// - Validate the checksum
	let mut hdr_region = vec![0; 8192];
	let len = fp.read(&mut hdr_region).unwrap();
	let saved_region = hdr_region.clone();
	let hdr_region = &mut hdr_region[..len];
	let saved_region = &saved_region[..len];
	{
		let hdr = find_header(hdr_region);

		if let Some(video_mode) = args.video_mode {
			hdr[8+0] |= 0x04;	// set `flags[2]`
			set_u32(&mut hdr[32..], 0);	// `mode_type` = 0 (Linear Graphics Mode)
			set_u32(&mut hdr[36..], video_mode.width);	// `width`
			set_u32(&mut hdr[40..], video_mode.height);	// `height`
			set_u32(&mut hdr[44..], 32);	// `depth`
		}

		let cksum_add = calculate_checksum(hdr);
		let new_cksum = get_u32(&hdr[8..]) + cksum_add;
		set_u32(&mut hdr[8..], new_cksum);
	}
	if saved_region != hdr_region {
		println!("Writing back");
		fp.seek(::std::io::SeekFrom::Start(0)).unwrap();
		fp.write_all(hdr_region).unwrap();
	}
	else {
		println!("No change");
	}
}

fn find_header(buf: &mut [u8]) -> &mut [u8] {
	let len = buf.len() / 4 * 4;
	let buf = &mut buf[..len];
	let mut ofs = None;
	for i in 0 .. buf.len() / 4
	{
		let buf = &mut buf[i * 4 ..];
		let v = get_u32(buf);
		if v == 0x1BADB002 {
			let cksum = calculate_checksum(buf);
			println!("Possible header @{:#x}: cksum={:#x}", i*4, cksum);
			{
				let mut it = buf.chunks(4).map(|v| get_u32(v));
				fn dump_hex(it: impl Iterator<Item=u32>) {
					print!(">");
					for v in it {
						print!(" {:8x}", v);
					}
					print!("\n");
				}
				dump_hex(it.by_ref().take(3));
				dump_hex(it.by_ref().take(5));
				dump_hex(it.by_ref().take(4));
			}
			if cksum == 0 {
				ofs = Some(i * 4);
				break
			}
		}
	}
	if let Some(ofs) = ofs {
		return &mut buf[ofs..];
	}
	else {
		panic!("Failed to find a valid header")
	}
}
fn get_u32(v: &[u8]) -> u32 {
	assert!(v.len() >= 4);
	u32::from_le_bytes([v[0], v[1], v[2], v[3],])
}
fn set_u32(buf: &mut [u8], v: u32) {
	assert!(buf.len() >= 4);
	let v = v.to_le_bytes();
	buf[..4].copy_from_slice(&v);
}
fn calculate_checksum(buf: &[u8]) -> u32 {
	// Checksum is only over the header fields (`magic`` and `flags`)
	buf.chunks(4).map(|v| get_u32(v)).take(3).fold(0, |a,b| a.wrapping_add(b))
}
struct Args {
	infile: ::std::path::PathBuf,
	video_mode: Option<VideoMode>,
}
impl Args {
	fn from_args() -> Self {
		let mut infile = None;
		let mut video_mode = None;
		let mut it = ::std::env::args().skip(1);
		while let Some(v) = it.next() {
			if v.starts_with("--") {
				match &v[..] {
				"--video-mode" => video_mode = Some(it.next().expect("Expected option").parse().expect("Invalid video mode")),
				v => panic!("Unexpected flag {:?}", v),
				}
			}
			else if v.starts_with("-") {
				for c in v.chars().skip(1) {
					match c {
					'V' => video_mode = Some(it.next().expect("Expected option").parse().expect("Invalid video mode")),
					c => panic!("Unexpected flag {:?}", c),
					}
				}
			}
			else {
				if infile.is_none() {
					infile = Some(::std::path::Path::new(&v).to_owned());
				}
				else {
					panic!("Too many free arguments");
				}
			}
		}
		Args {
			infile: infile.expect("Must pass an input file"),
			video_mode,
		}
	}
}
struct VideoMode {
	width: u32,
	height: u32,
}
impl ::std::str::FromStr for VideoMode {
	type Err = &'static str;
	fn from_str(v: &str) -> Result<Self,Self::Err> {
		let mut it = v.split('x');
		let w: u32 = it.next().unwrap().parse().map_err(|_| "Bad number")?;
		let h: u32 = it.next().ok_or("WxH[xD]")?.parse().map_err(|_| "Bad number")?;
		if let Some(_) = it.next() {
			return Err("Too many `x`");
		}
		Ok(VideoMode { width: w, height: h })
	}
}