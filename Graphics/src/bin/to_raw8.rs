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
	let im = im.to_luma();
	
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
	outfile.write(im.as_flat_samples().samples).unwrap();
}

