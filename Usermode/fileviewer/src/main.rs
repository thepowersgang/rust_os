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

	menu: (),
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

	
	//let path = "/system/1.txt";
	let path = "/sysroot/bin/fileviewer";
	let mut file = match ::syscalls::vfs::File::open(path, ::syscalls::vfs::FileOpenMode::ReadOnly)
		{
		Ok(v) => v,
		Err(e) => {
			kernel_log!("TOOD: Handle open error in fileviewer - {:?}", e);
			return ;
			},
		};


	// 1. Read a few lines and check if they're valid UTF-8 (just scanning)
	// - Limit the read to 1kb of data
	let use_hex = true;
	
	// 2. Select and populate the initial visualiser
	let root = Viewer::new(&mut file, use_hex);

	let mut window = ::wtk::Window::new_def("File viewer", &root).unwrap();
	window.set_title("File Viewer");

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

			menu: (),
			vscroll: ::wtk::ScrollbarV::new(),
			hscroll: ::wtk::ScrollbarH::new(),
			toggle_button: ::wtk::Button::new_boxfn( ::wtk::Colour::theme_body_bg(), |_,_| {} ),
			};

		if init_use_hex {
			let mut file = rv.file.borrow_mut();
			file.set_cursor(0);
			let _ = rv.hex.populate(&mut *file);
			//self.vscroll.set( Some(rv.hex.get_start(), filesize - rv.hex.get_capacity()) );
			//self.hscroll.set( None );
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
const MENU_HEIGHT: u32 = 16;
const SCROLL_SIZE: u32 = 16;
impl<'a> ::wtk::Element for Viewer<'a>
{
	fn resize(&self, width: u32, height: u32) {
		*self.dims.borrow_mut() = (width, height);

		self.menu.resize(width, MENU_HEIGHT);
		let body_width  = width - SCROLL_SIZE;
		let body_height = height - MENU_HEIGHT - SCROLL_SIZE;

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
			},
		}
		self.hscroll.resize(body_height, SCROLL_SIZE);
	}
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
		use wtk::geom::Rect;
		let (width, height) = (surface.width(), surface.height());
		assert_eq!( (width,height), *self.dims.borrow() );

		self.menu.render(surface.slice(Rect::new(0, 0,  width, MENU_HEIGHT)), force);
		let body_width  = width - SCROLL_SIZE;
		let body_height = height - MENU_HEIGHT - SCROLL_SIZE;
		self.vscroll.render(surface.slice(Rect::new(body_width, MENU_HEIGHT,  SCROLL_SIZE, body_height)), force);

		let body_view = surface.slice(Rect::new(0, MENU_HEIGHT,  body_width, body_height));
		match self.mode
		{
		ViewerMode::Hex  => self.hex.render(body_view, force),
		ViewerMode::Text => self.text.render(body_view, force),
		}
		self.hscroll.render(surface.slice(Rect::new(0, height - SCROLL_SIZE, body_width, SCROLL_SIZE)), force);
	}
	fn element_at_pos(&self, x: u32, y: u32) -> (&::wtk::Element, (u32,u32)) {
		let (width, height) = *self.dims.borrow();

		let vscroll_pos = (width - SCROLL_SIZE, MENU_HEIGHT);
		let hscroll_pos = (0, height - SCROLL_SIZE);

		if y < MENU_HEIGHT {
			self.menu.element_at_pos(x - 0, y - 0)
		}
		else if y < hscroll_pos.1 {
			if x > vscroll_pos.0 {
				self.vscroll.element_at_pos(x - vscroll_pos.0, y - vscroll_pos.1)
			}
			else {
				match self.mode
				{
				ViewerMode::Hex  => (&self.hex , (0,MENU_HEIGHT)),
				ViewerMode::Text => (&self.text, (0,MENU_HEIGHT)),
				}
			}
		}
		else {
			if x > vscroll_pos.0 {
				self.toggle_button.element_at_pos(x, y - hscroll_pos.1)
			}
			else {
				self.hscroll.element_at_pos(x - hscroll_pos.0, y - hscroll_pos.1)
			}
		}
	}
}

