// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// Simplistic console, used as a quick test case (fullscreen window)
#![feature(core_slice_ext,core_str_ext)]

#[macro_use]
extern crate syscalls;

use syscalls::Object;

mod terminal_surface;
mod terminal;

mod input;

use std::fmt::Write;

fn main() {
	use syscalls::gui::{Group,Window};
	use syscalls::threads::S_THIS_PROCESS;
	
	::syscalls::threads::wait(&mut [S_THIS_PROCESS.get_wait()], !0);
	::syscalls::gui::set_group( S_THIS_PROCESS.receive_object::<Group>(0).unwrap() );
	
	let window = Window::new("Console").unwrap();
	window.maximise();
	window.fill_rect(0,0, !0,!0, 0x33_00_00);   // A nice rust-like red :)
	let mut term = terminal::Terminal::new(&window, ::syscalls::gui::Rect::new(0,0, 1920,1080));
	{
		let _ = write!(&mut term, "Tifflin - Simple console\n");
		term.flush();
	}
	window.show();
	let mut input = input::InputStack::new();
	
	loop {
		// Bind to receive events relating to the window
		let mut events = [window.get_wait()];
		
		::syscalls::threads::wait(&mut events, !0);
	
		while let Some(ev) = window.pop_event()
		{
			kernel_log!("ev = {:?}", ev);
			match ev
			{
			::syscalls::gui::Event::KeyUp(kc) => {
				if let Some(buf) = input.handle_key(true, kc as u8, |s| term.write_str(s).unwrap())
				{
					kernel_log!("buf = {:?}", buf);
					term.write_str("\n").unwrap();
				}
				term.flush();
				window.redraw();
				},
			::syscalls::gui::Event::KeyDown(kc) => {
				input.handle_key(false, kc as u8, |_| ());
				},
			_ => {},
			}
		}
		
		window.check_wait(&events[0]);
	}
}

