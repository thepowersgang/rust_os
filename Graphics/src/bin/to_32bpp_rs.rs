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

fn main() {
	let args = Args::from_args();
	
	let im = image::open(&args.infile).expect("Can't open input file");
	let im = im.into_rgba8();
	
	let outfile = std::fs::File::create(&args.outfile).expect("Can't open output");
	
	save_as_rs(outfile, &im, &args.symbol).expect("Failed to write");
}

fn save_as_rs(mut outfile: impl std::io::Write, im: &image::RgbaImage, symbol: &str) -> std::io::Result<()>
{
	write!(outfile, "
		const {symbol}_DIMS: (u32,u32) = ({w},{h});\n
		static {symbol}_DATA: [u32; {npx}] = [
		", symbol=symbol, w=im.width(), h=im.height(), npx = im.width() * im.height())?;
	for (i,p) in Iterator::enumerate(im.pixels())
	{
		write!(outfile, "0x{:08x}, ", rgba_to_u32(*p))?;
		if i % 16 == 15 {
			write!(outfile, "\n")?;
		}
	}
	write!(outfile, "\n];\n")?;
	Ok( () )
}
