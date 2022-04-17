
#[derive(Debug)]
pub enum Error
{
	NotFound,
	NotExecutable,
	BadFormat,
	CorruptExecutable,
	BadArguments,
}