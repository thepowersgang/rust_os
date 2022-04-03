use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Args
{
	#[structopt(parse(from_os_str))]
	infile: std::path::PathBuf,
	#[structopt(parse(from_os_str))]
	outfile: std::path::PathBuf,
}


fn main()
{
	use std::io::Write;
	
	let args = Args::from_args();
	
	let im = image::open(&args.infile).expect("Can't open input file");
	
	let width = im.width();
	let height = im.height();

	let mut outfile = ::std::fs::File::create(&args.outfile).expect("Cannot open output file");

	outfile.write(b"\x7FR8M").unwrap();
	outfile.write(&[
		(width >> 0) as u8,
		(width >> 8) as u8,
		(height >> 0) as u8,
		(height >> 8) as u8,
		]).unwrap();

	match im
	{
	// Hack: If it's an RGBA image with the same value for non-alpha, use the alpha
	::image::DynamicImage::ImageRgba8(im) if {
		let mut it = im.pixels();
		let px = it.next().unwrap();
		it.all(|v| &v.0[..2] == &px.0[..2])
		} => {
			let v: Vec<_> = im.pixels().map(|v| v[3]).collect();
			outfile.write(&v).unwrap();
		},
	_ => {
		let im = im.into_luma8();
		outfile.write(im.as_flat_samples().samples).unwrap();
		}
	}
}

