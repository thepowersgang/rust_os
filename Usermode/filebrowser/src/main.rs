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
	::wtk::initialise();

	let mut root_handle = ::syscalls::vfs::Dir::open("/").unwrap();

	let mut fl = ::filelist::FileList::new();

	fl.populate(&mut root_handle);
	fl.on_chdir(|win, newdir| win.set_title(format!("Filesystem - {}", newdir.display())));
	fl.on_open(|_win, file_path| kernel_log!("TODO: Open path {}", file_path.display()));

	let mut window = ::wtk::Window::new_def("File browser", &fl).unwrap();
	window.set_title("Filesystem - /");

	window.focus(&fl);
	window.show();

	window.idle_loop();
}
