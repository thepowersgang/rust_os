//
//
//
///

extern crate wtk;
extern crate vec_ring;
#[macro_use(kernel_log)]
extern crate syscalls;
extern crate loader;

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

	let root_handle: ::syscalls::vfs::Dir = ::syscalls::threads::S_THIS_PROCESS.receive_object().unwrap();
	//let root_handle = ::syscalls::vfs::Dir::open("/").unwrap();

	let mut fl = ::filelist::FileList::new(&root_handle);

	fl.populate(&root_handle);
	fl.on_chdir(|win, newdir| win.set_title(format!("Filesystem - {}", newdir.display())));
	fl.on_open(|_win, file_path| view_file(file_path));

	let mut window = ::wtk::Window::new_def("File browser", &fl).unwrap();
	window.set_title("Filesystem - /");

	window.focus(&fl);
	window.show();

	window.idle_loop();
}

fn view_file(p: &::std::fs::Path) {
	panic!("filebrowser - view_file (post VFS rejig)");
	//let byte_args: &[&[u8]] = &[ p.as_ref(), ];
	//match ::loader::new_process(b"/sysroot/bin/fileviewer", byte_args)
	//{
	//Ok(app) => {
	//	app.send_obj( ::syscalls::gui::clone_group_handle() );
	//	},
	//Err(_e) => {},
	//}
}
