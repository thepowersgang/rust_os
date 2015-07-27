// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// Simplistic console, used as a quick test case (fullscreen window)

extern crate syscalls;

use syscalls::Object;

fn main() {
	use syscalls::gui::{Group,Window};
	
	::syscalls::gui::set_group( ::syscalls::threads::S_THIS_PROCESS.receive_object::<Group>(0).unwrap() );
	
	let window = Window::new("Console").unwrap();
	window.maximise();
	window.fill_rect(0,0, !0,!0, 0x0);
	window.show();
	
	loop {
		// Bind to receive events relating to the window
		let mut events = [window.get_wait()];
		
		::syscalls::threads::wait(&mut events, !0);
		
		window.check_wait(&events[0]);
	}
}

