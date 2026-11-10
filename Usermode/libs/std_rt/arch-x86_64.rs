
pub struct Backtrace(usize);
impl Backtrace {
	pub fn new() -> Backtrace {
		// SAFE: Just loads bp
		let bp = unsafe { 
			let bp: usize;
			::core::arch::asm!("mov {0}, rbp", lateout(reg) bp);
			bp
			};
		Backtrace(bp)
	}
}

impl ::core::fmt::Debug for Backtrace {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let mut bp = self.0 as u64;
		while let Some( (newbp, ip) ) = backtrace(bp)
		{
			try!( write!(f, " > {:#x}", ip) );
			bp = newbp;
		}
		Ok( () )
	}
}

/// Obtain the old RBP value and return address from a provided RBP value
fn backtrace(bp: u64) -> Option<(u64,u64)>
{
	if bp == 0 {
		return None;
	}
	if bp % 8 != 0 {
		return None;
	}
	//if ! ::memory::buf_valid(bp as *const (), 16) {
	//	return None;
	//}
	if bp >= (1<<47) {
		return None
	}
	
	// [rbp] = oldrbp, [rbp+8] = IP
	// SAFE: (uncheckable) Walks stack frames and may crash
	unsafe
	{
		let ptr: *const [u64; 2] = bp as usize as *const _;
		let newbp = (*ptr)[0];
		let newip = (*ptr)[1];
		// Check validity of output BP, must be > old BP (upwards on the stack)
		// - If not, return 0 (which will cause a break next loop)
		if newbp <= bp {
			Some( (0, newip) )
		}
		else {
			Some( (newbp, newip) )
		}
	}
}


