// Tifflin OS - login
// - By John Hodge (thePowersGang)
//
// Graphical user session root - Provides background, taskbar and menus

extern crate wtk;
extern crate r#async;
#[macro_use]
extern crate syscalls;

extern crate loader;

use syscalls::gui::KeyCode;
use wtk::ModifierKey;

macro_rules! imgpath {
		($p:expr) => {concat!("/system/Tifflin/shared/images/",$p)};
}

fn start_app_console() {
	start_app(&["/sysroot/bin/simple_console", "--windowed"], |_app| {
		//app.send_obj( "vfs", ::syscalls::vfs::root().clone() );
		});
}
fn start_app_filebrowser() {
	start_app(&["/sysroot/bin/filebrowser"], |app| {
		app.send_obj( "ro:/", ::syscalls::vfs::root().clone() );
		});
}
fn start_app_editor() {
	let path = "/system/1.txt";
	let f = ::syscalls::vfs::root().open_child_path(path.as_bytes()).expect("Couldn't open editor executable file")
		.into_file(::syscalls::vfs::FileOpenMode::ReadOnly).expect("Couldn't open file as readonly");
	start_app(&["/sysroot/bin/fileviewer", path], |app| {
		app.send_obj( "file", f );
		});
}

fn main()
{
	::wtk::initialise();


	let power_menu = {
		use wtk::menu::{Menu,Entry,Spacer};
		Menu::new("Power Menu", (
			Entry::new("Lock", 0, "", || {}),
			Entry::new("Logout", 1, "", || {}),
			Spacer,
			Entry::new("Restart", 0, "", || {}),
			Entry::new("Shut Down", 0, "", || {}),
			))
		};
	//power_menu.set_pos( Pos::new(0,20) );
	let system_menu = {
		use wtk::menu::{Menu,Entry,Spacer};
		Menu::new("System Menu", (
			Entry::new("CLI", 0, "Win-T", || start_app_console()),
			Spacer,
			Entry::new("Filesystem", 0, "Win-E", || start_app_filebrowser()),
			Entry::new("Text Editor", 5, "", || start_app_editor()),
			))
		};
	system_menu.set_pos( ::wtk::geom::Pos::new(0,20) );
	
	let background = {
		// Background image is "Ferris the crab" - credit to rustacean.net
		let img = ::wtk::image::RasterRGB::new_img(imgpath!("background.r24")).expect("Cannot load background");
		img
		};
	let mut win_background = {
		let mut win = ::wtk::Window::new("Background", &background, ::wtk::Colour::from_argb32(0x01346B), ()).unwrap();	// 01346B is Ferris's background colour
		win.maximise();
		win
		};
	
	let menubar = {
		let logo_button = ::wtk::Button::new(
			::wtk::image::RasterMonoA::new_img(imgpath!("menu.r8"), ::wtk::Colour::theme_text()).expect("Error loading menu icon"),
			|_,_| system_menu.show()
			);
		let taskbar = ();
		let clock_widget = ::std::cell::RefCell::new(::wtk::OwnedLabel::new("12:34".to_owned(), ::wtk::Colour::theme_text()));
		let power_button = ::wtk::Button::new(
			::wtk::image::RasterMonoA::new_img(imgpath!("power.r8"), ::wtk::Colour::theme_text()).expect("Error loading power icon"),
			|_button, _window| power_menu.show()
			);
		let options_button = ::wtk::Button::new(
			::wtk::image::RasterMonoA::new_img(imgpath!("options.r8"), ::wtk::Colour::theme_text()).expect("Error loading options icon"),
			|_button, _window| {}
			);
		::wtk::StaticBox::new_horiz((
			::wtk::BoxEle::fixed(20, logo_button),
			::wtk::BoxEle::expand(taskbar),
			::wtk::BoxEle::fixed(50, clock_widget),
			::wtk::BoxEle::fixed(20, power_button),
			::wtk::BoxEle::fixed(20, options_button),
			))
		};
	let mut win_menu = {
		let mut win = ::wtk::Window::new("SystemBar", &menubar, ::wtk::Colour::theme_text_bg(), ()).unwrap();
		win.set_pos(0, 0);
		let di = ::syscalls::gui::clone_group_handle().get_display_info();
		win.set_dims(di.total_width, 20);
		//win.taborder_add(0, &menubar.inner().0);
		//win.taborder_add(1, &menubar.inner().3);
		win
		};
	
	win_menu.add_shortcut_1( KeyCode::LeftGui, || system_menu.show() );
	win_menu.add_shortcut_1( KeyCode::RightGui, || system_menu.show() );
	win_menu.add_shortcut_2( ModifierKey::Gui, KeyCode::T, || start_app_console() );
	win_menu.add_shortcut_2( ModifierKey::Gui, KeyCode::E, || start_app_filebrowser() );

	win_background.show();
	win_menu.show();

	//let clock_timer = AsyncTimer::new_realtime( time::Realtime::now(), || {
	//		let mut cur_time = time::Realtime::now();
	//		// Update clock
	//		menubar.inner().2.inner().borrow_mut().set( cur_time.format("%H:%M").to_string() );
	//		// Re-schedule for the next minute
	//		Some(cur_time % time::Interval::minute(1) + time::Interval::minute(1))
	//		});

	::r#async::idle_loop(&mut [
		&mut win_background,
		&mut win_menu,
		&mut system_menu.waiter(),
		&mut power_menu.waiter(),
		]);

}

fn start_app<F>(args: &[&str], cb: F)
where
	F: FnOnce(&mut ::loader::ProtoProcess)
{
	kernel_log!("start_app(args={:?})", args);
	let fh = open_exec(args[0]);
	// SAFE: &str and &[u8] have the same representation
	let byte_args: &[&[u8]] = unsafe { ::std::mem::transmute(&args[1..]) };
	match ::loader::new_process(fh, args[0].as_bytes(), byte_args)
	{
	Ok(mut app) => {
		app.send_obj( "guigrp", ::syscalls::gui::clone_group_handle() );
		cb(&mut app);
		app.start();
		},
	Err(_e) => {},
	}
}

fn open_exec(path: &str) -> ::syscalls::vfs::File
{
	match ::syscalls::vfs::root().open_child_path(path.as_bytes())
	{
	Ok(v) => match v.into_file(::syscalls::vfs::FileOpenMode::Execute)
		{
		Ok(v) => v,
		Err(e) => panic!("Couldn't open '{}' as an executable file - {:?}", path, e),
		},
	Err(e) => panic!("Couldn't open executable '{}' - {:?}", path, e),
	}
}
