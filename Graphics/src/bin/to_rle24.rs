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
	
	//symbol: String,
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

	outfile.write(b"\x7FR\x18R").unwrap();
	outfile.write(&[
		(width >> 0) as u8,
		(width >> 8) as u8,
		(height >> 0) as u8,
		(height >> 8) as u8,
		]).unwrap();
	
	let mut raw_data: Vec<u32> = Vec::with_capacity( (width*height) as usize );
	for p in im.pixels()
	{
		raw_data.push( rgba_to_u32(*p) );
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
	f.write(&buf).unwrap();
}

