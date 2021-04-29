pub struct Backtrace(u8);
impl Backtrace {
	pub fn new() -> Backtrace {
		Backtrace(0)
	}
}
impl ::core::fmt::Debug for Backtrace {
	fn fmt(&self, _f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		Ok( () )
	}
}

