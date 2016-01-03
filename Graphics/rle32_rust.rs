

fn main()
{
	use ::std::io::{Write,Read};

	const ARGS_ERR: &'static str = "Required 4 arguments: NAME IN OUT WIDTH";
	let mut args = ::std::env::args();
	args.next();
	let name = args.next().expect(ARGS_ERR);
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

	const COUNT_MAX: usize = 0xFF;   const COUNT_TYPE: &'static str = "u8";
	//const COUNT_MAX: usize = 0xFFFF; const COUNT_TYPE: &'static str = "u16";

	write!(&mut outfile, "const {}_DIMS: (u32,u32) = ({},{});\n", name, width, height).unwrap();
	write!(&mut outfile, "struct RleRow(&'static [{}], &'static [u32]);\n", COUNT_TYPE).unwrap();
	write!(&mut outfile, "{}", r##"
impl RleRow {
	pub fn decompress(&self, dst: &mut [u32]) {
		let mut j = 0;
		for i in 0 .. self.0.len() {
			let (&c,&v) = unsafe { (self.0.get_unchecked(i), self.1.get_unchecked(i)) };
			for _ in 0 .. c {
				dst[j] = v;
				j += 1;
			}
		}
	}
}
"##).unwrap();
	write!(&mut outfile, "static {}_DATA: [ RleRow; {} ] = [\n", name, height).unwrap();
	
	let mut raw_data: Vec<u32> = vec![0; width];
	for _ in 0 .. height
	{
		let bytes = unsafe { ::std::slice::from_raw_parts_mut( &mut raw_data[0] as *mut _ as *mut u8, width * 4 ) };
		let len = infile.read(bytes).expect("Can't read from input");
		assert_eq!(len, bytes.len());

		let mut counts = vec![];
		let mut values = vec![];

		let mut count = 0;
		let mut value = 0;
		for &val in &raw_data {
			if val != value {
				counts.push( count );
				values.push( value );
				value = val;
				count = 0;
			}
			if count == COUNT_MAX {
				counts.push( count );
				values.push( value );
				count = 0;
			}
			count += 1;
		}
		if count > 0 {
			counts.push( count );
			values.push( value );
		}

		write!(&mut outfile, "\tRleRow(&[").unwrap();
		for count in counts {
			write!(&mut outfile, "{},", count).unwrap();
		}
		write!(&mut outfile, "], &[").unwrap();
		for value in values {
			let value = value.swap_bytes() >> 8;	// Endian flip and remove alpha
			write!(&mut outfile, "{:#x},", value).unwrap();
		}
		write!(&mut outfile, "]),\n").unwrap();
	}

	write!(&mut outfile, "];\n").unwrap();
}
