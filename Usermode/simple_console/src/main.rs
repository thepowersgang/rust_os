// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// Simplistic console, used as a quick test case (fullscreen window)

extern crate tifflin_syscalls;

fn main() {
	let window = ::tifflin_syscalls::gui::new_window("Console").unwrap();
	window.maximise();
	window.fill_rect(0,0, !0,!0, 0x0);
	window.show();
	
	loop {
		// Bind to receive events relating to the window
		window.bind_event(1);
		match ::tifflin_syscalls::wait_event()
		{
		0 => {
			// Kernel event
			},
		1 => {
			// Main window event
			},
		}
	}
}

