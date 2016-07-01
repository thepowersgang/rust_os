
fn main()
{
	use ::std::io::{Write,Read};

	const ARGS_ERR: &'static str = "Required 4 arguments: NAME IN OUT WIDTH";
	let mut args = ::std::env::args();
	args.next();
	let infile_path = args.next().expect(ARGS_ERR);
	let outfile_path = args.next().expect(ARGS_ERR);
	let width: usize = ::std::str::FromStr::from_str( &args.next().expect(ARGS_ERR) ).expect("WIDTH is invalid");

	let mut infile = ::std::fs::File::open(&infile_path).expect("Cannot open input file");
	let len = infile.metadata().expect("Can't get input file metatadata").len();

	if len % width as u64 != 0 {
		panic!("Input file isn't a multiple of width");
	}

	let height = (len / 4 / width as u64) as usize;

	let mut outfile = ::std::fs::File::create(&outfile_path).expect("Cannot open output file");

	outfile.write(b"\x7FR\x18R");
	outfile.write(&[
		(width >> 0) as u8,
		(width >> 8) as u8,
		(height >> 0) as u8,
		(height >> 8) as u8,
		]);
	
	let mut raw_data: Vec<u32> = vec![0; width*height];
	{
		let bytes = unsafe { ::std::slice::from_raw_parts_mut( &mut raw_data[0] as *mut _ as *mut u8, width * height * 4 ) };
		let len = infile.read(bytes).expect("Can't read from input");
		assert_eq!(len, bytes.len());
	}

	const COUNT_MAX: usize = 255;
	
	let mut count = 0;
	let mut value = 0;
	for &val in &raw_data {
		if val != value || count == COUNT_MAX {
			write_count_px( &mut outfile, count as u8, value );

			value = val;
			count = 0;
		}
		count += 1;
	}
	if count > 0 {
		write_count_px( &mut outfile, count as u8, value );
	}
}

fn write_count_px<F: ::std::io::Write>(f: &mut F, count: u8, val: u32) {
	let buf = [
		count,
		(val & 0xFF) as u8,
		(val >>  8 & 0xFF) as u8,
		(val >> 16 & 0xFF) as u8,
		//(val >> 32 & 0xFF) as u8,
		];
	f.write(&buf);
}

