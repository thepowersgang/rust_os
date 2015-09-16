
pub struct IRQHandle(u32);
impl Default for IRQHandle {
	fn default() -> IRQHandle { IRQHandle(!0) }
}

pub fn bind_gsi(gsi: usize, handler: fn(*const()), info: *const ()) -> Result<IRQHandle,()> {
	Ok( IRQHandle(gsi as u32) )
}

