
#[derive(Default)]
pub struct IRQHandle;
#[derive(Debug)]
pub struct BindError;

pub fn bind_gsi(idx: usize, handler: fn(*const ()), info: *const ()) -> Result<IRQHandle,BindError> {
	Ok(IRQHandle)
}

