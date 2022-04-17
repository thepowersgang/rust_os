// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// Simplistic console, used as a quick test case (fullscreen window)
#[macro_use]
extern crate syscalls;

extern crate cmdline_words_parser;

extern crate wtk;
extern crate r#async;

use wtk::Colour;

mod terminal_element;
mod input;


trait Terminal
{
	fn set_foreground(&self, col: ::wtk::Colour);
	fn flush(&self);
	fn cur_col(&self) -> usize;
	fn delete_left(&self);
	fn delete_right(&self);
	fn cursor_left(&self);

	fn write_str(&self, s: &str);
	fn write_fmt(&self, args: ::std::fmt::Arguments) {
		struct Adapter<'a, T: 'a + ?Sized + Terminal>(&'a T);
		impl<'a,T: ?Sized + 'a + Terminal> ::std::fmt::Write for Adapter<'a, T> {
			fn write_str(&mut self, s: &str) -> ::std::fmt::Result {
				self.0.write_str(s);
				Ok( () )
			}
		}
		let _ = ::std::fmt::write(&mut Adapter(self), args);
	}
}

fn main()
{
	let mut maximised = false;
	// TODO: Create a clone of getopts/docopt for this work
	for arg in ::std::env::args_os().skip(1) {
		match arg.as_bytes()
		{
		b"--maximised" => {maximised = true;},
		_ => {
			kernel_log!("Unknown arg {:?}", arg);
			},
		}
	}
	
	::wtk::initialise();

	let mut shell = ShellState::new();
	let mut input = input::InputStack::new();
	let term_ele = ::terminal_element::TerminalElement::new(
		|_window, term, ev|
		if let Some(buf) = input.handle_event(ev, |a| render_input(term, a))
		{
			kernel_log!("buf = {:?}", buf);
			term.write_str("\n");

			// XXX: Lazy option really... would maybe be cleaner to either have a flag in `shell` or just explicitly
			//      exit when the exit command is invoked
			if buf == "exit" {
				::syscalls::threads::exit(0);
			}

			shell.handle_command(term, buf);
			// - If the command didn't print a newline, print one for it
			if term.cur_col() != 0 {
				term.write_str("\n");
			}
			// New prompt
			term.write_str("> ");
		}
		);

	// Create maximised window
	let decorator = if maximised { None } else { Some(::wtk::decorator::Standard::default()) };
	let mut window = ::wtk::Window::new("Console", &term_ele, ::wtk::Colour::from_argb32(0x330000), decorator).unwrap();
	if maximised {
		window.maximise();
	}
	else {
		window.set_pos(50, 50);
		window.set_dims(160*8+10, 25*16+20);
		window.set_title("Console");
	}

	// Create terminal
	// Print header
	{
		let mut buf = [0; 128];
		term_ele.set_foreground( Colour::from_argb32(0x00FF00) );
		let _ = write!(term_ele, "{}\n",  ::syscalls::get_text_info(::syscalls::TEXTINFO_KERNEL, 0, &mut buf));	// Kernel 0: Version line
		term_ele.set_foreground( Colour::from_argb32(0xFFFF00) );
		let _ = write!(term_ele, " {}\n", ::syscalls::get_text_info(::syscalls::TEXTINFO_KERNEL, 1, &mut buf));	// Kernel 1: Build line
		term_ele.set_foreground( Colour::from_argb32(0xFFFFFF) );
		let _ = write!(term_ele, "Simple console\n");
	}
	// Initial prompt
	term_ele.write_str("> ");

	window.focus(&term_ele);
	window.show();

	::r#async::idle_loop(&mut [
		&mut window,
		]);
}


// Render callback for input stack
fn render_input<T: Terminal>(term: &T, action: input::Action)
{
	use input::Action;
	match action
	{
	Action::Backspace => term.delete_left(),
	Action::Delete => term.delete_right(),
	Action::Puts(s) => term.write_str(s),
	}
}

struct ShellState
{
	/// Root directory handle
	root_handle: ::syscalls::vfs::Dir,

	/// Current working directory, relative to /
	cwd_rel: String,
}


macro_rules! print {
	($term:expr, $($t:tt)*) => ({let _ = write!($term, $($t)*);});
}

impl ShellState
{
	pub fn new() -> ShellState {
		ShellState {
			cwd_rel: Default::default(),
			root_handle: ::syscalls::vfs::root().clone(),
			}
	}
	/// Handle a command
	pub fn handle_command<T: Terminal>(&mut self, term: &T, mut cmdline: String)
	{
		let mut args = cmdline_words_parser::parse_posix(&mut cmdline);
		match args.next()
		{
		None => {},
		// 'pwd' - Print working directory
		Some("pwd") => print!(term, "/{}", self.cwd_rel),
		// 'cd' - Change directory
		Some("cd") =>
			if let Some(p) = args.next()
			{
				print!(term, "TODO: cd '{}'", p);
			}
			else
			{
				self.cwd_rel = String::new();
			},
		// 'ls' - Print the contents of a directory
		Some("ls") =>
			if let Some(dir) = args.next()
			{
				// TODO: Parse 'dir' as relative correctly
				command_ls(term, &self.root_handle, dir);
			}
			else
			{
				command_ls(term, &self.root_handle, &format!("/{}", self.cwd_rel));
			},
		// 'cat' - Dump the contents of a file
		// TODO: Implement
		Some("cat") => print!(term, "TODO: cat"),
		// 'echo' - Prints all arguments space-separated
		Some("echo") =>
			while let Some(v) = args.next() {
				print!(term, "{} ", v);
			},
		Some("help") => {
			print!(term, "Builtins: pwd, cd, ls, cat, help, echo");
			},
		Some(cmd @_) => {
			print!(term, "Unkown command '{}'", cmd);
			},
		}
	}
}

/// List the contents of a directory
fn command_ls<T: ::Terminal>(term: &T, root: &::syscalls::vfs::Dir, path: &str)
{
	use syscalls::vfs::{NodeType, FileOpenMode};
	let handle = match root.open_child_path(path)
		{
		Ok(v) => match v.into_dir()
			{
			Ok(v) => v,
			Err(e) => {
				print!(term, "Unable to open '{}': {:?}", path, e);
				return ;
				},
			},
		Err(e) => {
			print!(term, "Unable to open '{}': {:?}", path, e);
			return ;
			},
		};
	let mut iter = handle.enumerate().unwrap();
	
	let mut buf = [0; 256];
	loop
	{
		let name_bytes = match iter.read_ent(&mut buf)
			{
			Ok(Some(v)) => v,
			Ok(None) => break,
			Err(e) => {
				print!(term, "Read error: {:?}", e);
				return ;
				},
			};

		let name = ::std::str::from_utf8(name_bytes).expect("Filename not utf-8");

		print!(term, "- {}", name);

		let file_node = match handle.open_child(name)
			{
			Ok(v) => v,
			Err(e) => {
				print!(term, "(Error: {:?})\n", e);
				continue ;
				},
			};
		match file_node.class()
		{
		NodeType::File => {
			let size = file_node.into_file(FileOpenMode::ReadOnly).and_then(|h| Ok(h.get_size())).unwrap_or(0);
			print!(term, " ({})", size);
			},
		NodeType::Dir => print!(term, "/"),
		NodeType::Symlink => {
			let mut link_path_buf = [0; 256];
			let dest = match file_node.into_symlink().and_then(|h| h.read_target(&mut link_path_buf))
				{
				Ok(v) => v,
				Err(e) => { print!(term, "(Error: {:?})\n", e); continue ; },
				};
			print!(term, " => {:?}", ::std::str::from_utf8(dest));
			},
		NodeType::Special => print!(term, "*"),
		}
		print!(term, "\n");
	}
}


/// Trait to provde 'is_combining', used by render code
pub trait UnicodeCombining
{
	fn is_combining(&self) -> bool;
}

impl UnicodeCombining for char
{
	fn is_combining(&self) -> bool
	{
		match *self as u32
		{
		// Ranges from wikipedia:Combining_Character
		0x0300 ..= 0x036F => true,
		0x1AB0 ..= 0x1AFF => true,
		0x1DC0 ..= 0x1DFF => true,
		0x20D0 ..= 0x20FF => true,
		0xFE20 ..= 0xFE2F => true,
		_ => false,
		}
	}
}

