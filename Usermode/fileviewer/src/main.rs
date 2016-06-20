// Tifflin OS File Viewer
// - By John Hodge (thePowersGang)
//
//! Basic file viewer (text/hex)

extern crate wtk;
#[macro_use(kernel_log)]
extern crate syscalls;

mod hexview;
mod textview;


struct Viewer<'a>
{
	dims: ::std::cell::RefCell<(u32, u32)>,
	file: ::std::cell::RefCell<&'a mut ::syscalls::vfs::File>,
	mode: ViewerMode,

	vscroll: ::wtk::ScrollbarV,
	hscroll: ::wtk::ScrollbarH,
	hex: ::hexview::Widget,
	text: ::textview::Widget,
	toggle_button: ::wtk::ButtonBcb<'static, ::wtk::Colour>,
}
enum ViewerMode {
	Hex,
	Text,
}

fn main()
{
	::wtk::initialise();
	let mut file: ::syscalls::vfs::File = match ::syscalls::threads::S_THIS_PROCESS.receive_object("file")
		{
		Ok(v) => v,
		Err(e) => {
			kernel_log!("TOOD: Handle open error in fileviewer - {:?}", e);
			return ;
			},
		};

	for a in ::std::env::args_os() {
		kernel_log!("arg = {:?}", a);
	}

	let mut args = ::std::env::args_os().skip(0);
	let path = args.next();
	let path: Option<&::std::ffi::OsStr> = path.as_ref().map(|x| x.as_ref());
	let path = path.unwrap_or( ::std::ffi::OsStr::new(b"-") );


	// 1. Read a few lines and check if they're valid UTF-8 (just scanning)
	// - Limit the read to 1kb of data
	let use_hex = true;
	
	// 2. Select and populate the initial visualiser
	let root = Viewer::new(&mut file, use_hex);

	let mut window = ::wtk::Window::new_def("File viewer", &root).unwrap();
	window.set_title( format!("File Viewer - {:?}", path) );

	window.focus(&root);
	window.set_dims(root.min_width(), 150);
	window.set_pos(150, 100);
	window.show();

	window.idle_loop();
}

impl<'a> Viewer<'a>
{
	fn new(file: &'a mut ::syscalls::vfs::File, init_use_hex: bool) -> Viewer<'a> {
		let rv = Viewer {
			dims: ::std::cell::RefCell::new( (0,0) ),
			file: ::std::cell::RefCell::new(file),

			mode: if init_use_hex { ViewerMode::Hex } else { ViewerMode::Text },
			hex: ::hexview::Widget::new(),
			text: ::textview::Widget::new(),

			vscroll: ::wtk::ScrollbarV::new(),
			hscroll: ::wtk::ScrollbarH::new(),
			toggle_button: ::wtk::Button::new_boxfn( ::wtk::Colour::theme_body_bg(), |_,_| {} ),
			};

		if init_use_hex {
			let mut file = rv.file.borrow_mut();
			file.set_cursor(0);
			let _ = rv.hex.populate(&mut *file);
		}
		else {
			// 1. Calculate number of lines and maximum width?
			/*
			let mut file = rv.file.borrow_mut();
			
			let mut n_lines = 0;
			let mut max_len = 0;
			for line in file.split(b'\n') {
				max_len = ::std::cmp::max(max_len, line.len());
				n_lines += 1;
			}
			*/
			// 2. Pre-fill `n` lines
		}
		
		rv
	}

	pub fn min_width(&self) -> u32 {
		SCROLL_SIZE + self.hex.min_width() + 2*2
	}
}
const SCROLL_SIZE: u32 = 16;
impl<'a> ::wtk::Element for Viewer<'a>
{
	fn resize(&self, width: u32, height: u32) {
		*self.dims.borrow_mut() = (width, height);

		let body_width  = width - SCROLL_SIZE;
		let body_height = height - SCROLL_SIZE;

		self.vscroll.resize(SCROLL_SIZE, body_height);
		match self.mode
		{
		ViewerMode::Hex  => {
			use std::io::Seek;
			self.hex.resize(body_width, body_height);
			let ofs = self.hex.get_start();
			let mut file = self.file.borrow_mut();
			let _ = file.seek(::std::io::SeekFrom::Start(ofs)).and_then(|_| self.hex.populate(&mut *file));
			},
		ViewerMode::Text => {
			self.text.resize(body_width, body_height);

			let mut file = self.file.borrow_mut();
			let _ = self.text.populate(&mut *file);
			},
		}
		self.hscroll.resize(body_width, SCROLL_SIZE);

		// Reposition the scroll elements
		let file = self.file.borrow();
		let filesize = file.get_size();
		if filesize > usize::max_value() as u64 {
			// - Disable vertical scroll if the filesize is > usize
			self.vscroll.set_bar( None );
		}
		else if filesize <= self.hex.get_capacity() as u64 {
			self.vscroll.set_bar( Some( (0,0) ) );
		}
		else {
			self.vscroll.set_bar( Some( (filesize as usize, self.hex.get_capacity() as usize) ) );
		}
		self.vscroll.set_pos( 0 );
		self.hscroll.set_bar( None );
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
		use wtk::geom::Rect;
		let (width, height) = (surface.width(), surface.height());
		assert_eq!( (width,height), *self.dims.borrow() );

		let body_width  = width - SCROLL_SIZE;
		let body_height = height - SCROLL_SIZE;
		self.vscroll.render(surface.slice(Rect::new(body_width, 0,  SCROLL_SIZE, body_height)), force);

		let body_view = surface.slice(Rect::new(0, 0,  body_width, body_height));
		match self.mode
		{
		ViewerMode::Hex  => self.hex.render(body_view, force),
		ViewerMode::Text => self.text.render(body_view, force),
		}
		self.hscroll.render(surface.slice(Rect::new(0, height - SCROLL_SIZE, body_width, SCROLL_SIZE)), force);
	}
	fn with_element_at_pos(&self, pos: ::wtk::geom::PxPos, dims: ::wtk::geom::PxDims, f: ::wtk::WithEleAtPosCb) -> bool {
		let x = pos.x.0;
		let y = pos.y.0;
		let (width, height) = (dims.w.0, dims.h.0);

		let body_dims = ::wtk::geom::PxDims::new( width - SCROLL_SIZE, height - SCROLL_SIZE );
		let vscroll_pos = ::wtk::geom::PxPos::new(body_dims.w.0, 0);
		let hscroll_pos = ::wtk::geom::PxPos::new(0, body_dims.h.0);

		if y < hscroll_pos.y.0 {
			if x > vscroll_pos.x.0 {
				self.vscroll.with_element_at_pos(pos - vscroll_pos, ::wtk::geom::PxDims::new(SCROLL_SIZE, body_dims.h.0), f)
			}
			else {
				match self.mode
				{
				ViewerMode::Hex  => self.hex.with_element_at_pos(pos, body_dims, f),
				ViewerMode::Text => self.text.with_element_at_pos(pos, body_dims, f),
				}
			}
		}
		else {
			if x > body_dims.w.0 {
				self.toggle_button.with_element_at_pos(pos - body_dims.bottomright(), ::wtk::geom::PxDims::new(SCROLL_SIZE, SCROLL_SIZE), f)
			}
			else {
				self.hscroll.with_element_at_pos(pos - hscroll_pos, ::wtk::geom::PxDims::new(body_dims.w.0, SCROLL_SIZE), f)
			}
		}
	}
}

