
#[derive(Debug)]
pub struct Backtrace;
impl Backtrace {
	pub fn new() -> Backtrace {
		::syscalls::raw::trigger_panic()
	}
}
