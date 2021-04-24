
// Idea on a method to allow hardware (e.g. network cards) to recieve pointers into the stack without them being invalidated during semi-async DMA

struct DelayDrop<T>
{
	p: &T,
	can_drop: &AtomicBool,
}
impl<T> Deref for DelayDrop<T>
{
}

impl<T> DelayDrop<T>
{
	unsafe fn new(v: &T, can_drop: &AtomicBool) -> DelayDrop<T> {
	}
	
	fn scoped<U>(v: &T, cb: impl FnOnce(DelayDrop<T>)->U) -> U {
		let flag = Default::default();
		let v = Self::new(v, &flag);
		cb(v);
		todo!("Wait for `flag` to go high")
	}

	unsafe fn set_can_drop(self) {
		self.can_drop.store(true, SeqCst);
		mem::forget(self)
	}
}
impl<T> Drop for DelayDrop<T>
{
	fn drop(&mut self) {
		panic!("Can't let DelayDrop<T> drop")
	}
}

