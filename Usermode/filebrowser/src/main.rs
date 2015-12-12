//
//
//
///

extern crate wtk;
extern crate vec_ring;
#[macro_use(kernel_log)]
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

	let mut root_handle = ::syscalls::vfs::Dir::open("/").unwrap();

	let mut fl = ::filelist::FileList::new();

	fl.populate(&mut root_handle);
	fl.on_chdir(|win, newdir| win.set_title(format!("Filesystem - {}", newdir.display())));

	let mut window = ::wtk::Window::new_def("File browser", &fl).unwrap();
	window.set_title("Filesystem - /");

	window.focus(&fl);
	window.show();

	window.idle_loop();
}
