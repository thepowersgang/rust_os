
pub struct ElfModuleHandle;

pub fn load_executable(path: &str) -> ElfModuleHandle
{
	unimplemented!();
}

impl ElfModuleHandle
{
	pub fn get_entrypoint(&self) -> fn(&[&str]) -> ! {
		unimplemented!();
	}
}

