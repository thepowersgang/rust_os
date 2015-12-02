//
//
//
///

pub struct FileList
{
	scroll_pos: usize,
	files: Vec<FileEnt>
}

struct FileEnt;


impl FileList
{
	pub fn new() -> FileList
	{
		FileList {
			scroll_pos: 0,
			files: Vec::new(),
		}
	}
	

	pub fn populate(&mut self, dir: &::syscalls::vfs::Dir) {
		// TODO: Populate the file list from this directory
	}

	/// Bind to "Opening" a file (double-click or select+enter)
	pub fn bind_open<F>(&mut self, f: F)
	where
		F: FnMut(&mut FileList, &::std::fs::Path)
	{
	}
}

impl ::wtk::Element for FileList
{
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
	}
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::wtk::Element, (u32,u32)) {
		(self, (0,0))
	}
}

