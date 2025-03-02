fn main() {
	use ::std::path::Path;
	let out = ::std::env::var_os("OUT_DIR").unwrap();
	let out = Path::new(&out);
	to_rle32_rs(&out.join("panic.rs"), Path::new("../PanicImageNA.png"));
	to_raw32_rs(&out.join("logo.rs" ), Path::new("../TifflinLogoV1-128.png"));
}

fn to_raw32_rs(out_path: &::std::path::Path, in_path: &::std::path::Path)
{
	use std::io::Write;
	let im = image::open(in_path).expect("Can't open input file");
	let im = im.into_rgba8();
	let mut outfile = ::std::fs::File::create(out_path).expect("Cannot open output file");
	write!(outfile, "
		pub const DIMS: (u32,u32) = ({w},{h});\n
		pub static DATA: [u32; {npx}] = [
		", w=im.width(), h=im.height(), npx = im.width() * im.height()
		).unwrap();
	for (i,p) in Iterator::enumerate(im.pixels())
	{
		write!(outfile, "0x{:08x}, ", rgba_to_u32(*p)).unwrap();
		if i % 16 == 15 {
			write!(outfile, "\n").unwrap();
		}
	}
	write!(outfile, "\n];\n").unwrap();
}

fn to_rle32_rs(out_path: &::std::path::Path, in_path: &::std::path::Path)
{
	use std::io::Write;

	let im = image::open(in_path).expect("Can't open input file");
	let im = im.into_rgba8();
	
	let width = im.width();
	let height = im.height();

	let mut outfile = ::std::fs::File::create(out_path).expect("Cannot open output file");

	const COUNT_MAX: usize = 0xFF;

	write!(&mut outfile, "pub const DIMS: (u32,u32) = ({},{});\n", width, height).unwrap();
	write!(&mut outfile, "pub static DATA: [ super::RleRow; {} ] = [\n", height).unwrap();
	
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

		write!(&mut outfile, "\tsuper::RleRow(&[").unwrap();
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

fn rgba_to_u32(p: image::Rgba<u8>) -> u32 {
	(p.0[0] as u32) << 0
		| (p.0[1] as u32) << 8
		| (p.0[2] as u32) << 16
		| (p.0[3] as u32) << 24
}

