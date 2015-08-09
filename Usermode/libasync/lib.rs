
#[macro_use]
extern crate syscalls;

// TODO: Move these to another crate
pub trait WaitController
{
	fn get_count(&self) -> usize;
}

pub fn idle_loop(items: &mut [&mut WaitController])
{
	let mut objects = Vec::new();
	loop {
		let count = items.iter().fold(0, |sum,ctrlr| sum + ctrlr.get_count());
		objects.reserve( count );

		::syscalls::threads::wait(&mut objects, !0);

		objects.clear();
	}
}

