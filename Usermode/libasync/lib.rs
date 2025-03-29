// Tifflin OS - Asynchronous common interface
// - By John Hodge (thePowersGang)
//
//
//! Asynchronous waiting support

//#[macro_use]
extern crate syscalls;

/// Trait for types that can be used for 'idle_loop'
pub trait WaitController
{
	/// Return the number of times `populate` will call `cb`
	fn get_count(&self) -> usize;
	/// Populate a list of objects
	fn populate(&self, cb: &mut dyn FnMut(::syscalls::WaitItem));
	/// Called when the thread wakes up for any reason, `events` points to the `get_count()` objects created by `populate`
	fn handle(&mut self, events: &[::syscalls::WaitItem]);
}

/// Idle, handling events on each WaitController passed
pub fn idle_loop<'a,'b,'c>(items: &'a mut [&'b mut (dyn WaitController+'c)])
{
	let mut objects = Vec::new();
	loop {
		let count = items.iter().fold(0, |sum,ctrlr| sum + ctrlr.get_count());
		objects.reserve( count );

		for ctrlr in items.iter() {
			ctrlr.populate(&mut |wi| objects.push(wi));
		}

		::syscalls::threads::wait(&mut objects, !0);

		let mut ofs = 0;
		for ctrlr in items.iter_mut()
		{
			let num = ctrlr.get_count();
			ctrlr.handle( &objects[ofs .. ofs + num] );
			ofs += num;
		}

		objects.clear();
	}
}

