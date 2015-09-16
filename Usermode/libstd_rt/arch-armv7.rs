

pub struct Backtrace(usize);
impl Backtrace {
	pub fn new() -> Backtrace {
		// SAFE: Just loads lr
		let lr = unsafe { 
			let lr: usize;
			asm!("mov $0, lr" : "=r" (lr));
			lr
			};
		Backtrace(lr)
	}
}

impl ::core::fmt::Debug for Backtrace {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		try!( write!(f, " > {:#x}", self.0) );
		Ok( () )
	}
}

