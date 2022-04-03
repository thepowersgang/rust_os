use structopt::StructOpt;
use graphics::rgba_to_u32;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Args
{
	#[structopt(parse(from_os_str))]
	infile: std::path::PathBuf,
	#[structopt(parse(from_os_str))]
	outfile: std::path::PathBuf,
	
	symbol: String,
}

fn main()
{
	use std::io::Write;
	
	let args = Args::from_args();
	
	let im = image::open(&args.infile).expect("Can't open input file");
	let im = im.into_rgba8();
	
	let width = im.width();
	let height = im.height();

	let mut outfile = ::std::fs::File::create(&args.outfile).expect("Cannot open output file");

	const COUNT_MAX: usize = 0xFF;   const COUNT_TYPE: &'static str = "u8";
	//const COUNT_MAX: usize = 0xFFFF; const COUNT_TYPE: &'static str = "u16";

	write!(&mut outfile, "const {}_DIMS: (u32,u32) = ({},{});\n", args.symbol, width, height).unwrap();
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
	write!(&mut outfile, "static {}_DATA: [ RleRow; {} ] = [\n", args.symbol, height).unwrap();
	
	let mut raw_data: Vec<u32> = Vec::with_capacity(width as usize);
	for r in im.rows()
	{
		raw_data.clear();
		for p in r
		{
			raw_data.push( rgba_to_u32(*p) );
		}
		
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
