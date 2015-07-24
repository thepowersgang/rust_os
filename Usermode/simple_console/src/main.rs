// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// Simplistic console, used as a quick test case (fullscreen window)

extern crate tifflin_syscalls;

fn main() {
	let window = ::tifflin_syscalls::gui::Window::new("Console").unwrap();
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

