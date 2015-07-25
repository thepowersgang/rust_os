// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// Simplistic console, used as a quick test case (fullscreen window)

extern crate tifflin_syscalls;

use tifflin_syscalls::Object;

fn main() {
	use tifflin_syscalls::gui::{Group,Window};
	
	//::tifflin_syscalls::gui::set_group( ::tifflin_syscalls::receive_object::<Group>(0) );
	
	let window = Window::new("Console").unwrap();
	window.maximise();
	window.fill_rect(0,0, !0,!0, 0x0);
	window.show();
	
	loop {
		// Bind to receive events relating to the window
		let mut events = [window.get_wait()];
		
		::tifflin_syscalls::threads::wait(&mut events, !0);
		
		window.check_wait(&events[0]);
	}
}

