//
//
//
///
use std::ffi::OsString;
use listview::ListView;
use std::cell::RefCell;

pub struct FileList
{
	scroll_pos: usize,
	files: Vec<FileEnt>,
	
	// Stores `String` for two reasons: 1. Avoids borrow hell, 2. UTF-8ness
	list: RefCell<ListView<[&'static str; 2], (&'static str, String)>>,
}

struct FileEnt
{
	name: OsString,
}
impl FileEnt
{
	fn new(_dir: &::syscalls::vfs::Dir, name: &[u8]) -> FileEnt {
		FileEnt {
			name: OsString::from(name),
		}
	}
	fn type_str(&self) -> &'static str {
		"d"
	}
	fn name_string(&self) -> String {
		self.name.to_str_lossy().into_owned()
	}
}

impl FileList
{
	pub fn new() -> FileList
	{
		FileList {
			scroll_pos: 0,
			files: Vec::new(),
			list: RefCell::new(ListView::new(["T", "Filename"])),
		}
	}
	

	pub fn populate(&mut self, dir: &mut ::syscalls::vfs::Dir) {
		self.files = Vec::new();
		let mut namebuf = [0; 512];
		while let Ok(name) = dir.read_ent(&mut namebuf)
		{
			if name.len() == 0 {
				break;
			}
			
			self.files.push( FileEnt::new(dir, name) );
		}
		// TODO: Tell the list to clear
		self.list.borrow_mut().clear();
	}

	/// Bind to "Opening" a file (double-click or select+enter)
	pub fn bind_open<F>(&mut self, f: F)
	where
		F: FnMut(&mut FileList, &::std::fs::Path)
	{
		//self.list.borrow_mut().bind_open(|idx| );
	}
}

impl ::wtk::Element for FileList
{
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
		let mut list = self.list.borrow_mut();
		list.maybe_resize( surface.height(), |ofs| {
			if let Some(f) = self.files.get(ofs)
			{
				Some( (ofs, (f.type_str(), f.name_string())) )
			}
			else
			{
				None
			}
			});
		list.render(surface, force);
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::wtk::Element, (u32,u32)) {
		(self, (0,0))
	}
}

