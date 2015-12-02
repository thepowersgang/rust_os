//
//
//
///

extern crate wtk;
extern crate vec_ring;
extern crate syscalls;

mod listview;
mod filelist;

mod iterx {
	pub fn zip<A: IntoIterator, B: IntoIterator>(a: A, b: B) -> ::std::iter::Zip<A::IntoIter, B::IntoIter> {
		Iterator::zip(a.into_iter(), b.into_iter())
	}
}

fn main()
{
	use wtk::Colour;

	::wtk::initialise();

	let root_handle = ::syscalls::vfs::Dir::open("/").unwrap();

	let mut fl = ::filelist::FileList::new();
	fl.bind_open(|fl, item_name| {
		});

	fl.populate(&root_handle);

	let mut window = ::wtk::Window::new_def("File browser", &fl).unwrap();


	window.focus(&fl);
	window.show();

	window.idle_loop();
}
