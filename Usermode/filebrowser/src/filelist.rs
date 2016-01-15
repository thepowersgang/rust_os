//
//
//
///
use std::ffi::{OsString,OsStr};
use listview::ListView;
use std::cell::RefCell;
use std::fs::Path;
use wtk::WindowTrait;

pub struct FileList<'a>
{
	on_open: Box<Fn(&mut WindowTrait, &Path) + 'a>,
	on_chdir: Box<Fn(&mut WindowTrait, &Path) + 'a>,

	cur_paths: RefCell<Vec<OsString>>,
	
	list: ListView<[&'static str; 2], FileEnt>,
}

impl<'a> FileList<'a>
{
	pub fn new() -> FileList<'a>
	{
		FileList {
			on_open: Box::new(|_,_|()),
			on_chdir: Box::new(|_,_|()),
			cur_paths: Default::default(),
			list: ListView::new(["T", "Filename"]),
		}
	}
	

	pub fn populate(&self, dir: &mut ::syscalls::vfs::Dir) {
		let mut namebuf = [0; 512];
		self.list.clear();
		while let Ok(Some(name)) = dir.read_ent(&mut namebuf)
		{
			self.list.append_item( FileEnt::new(dir, name) );
		}
	}

	/// Bind to "Opening" a file (double-click or select+enter)
	pub fn on_open<F: 'a>(&mut self, f: F)
	where
		F: Fn(&mut ::wtk::WindowTrait, &Path)
	{
		self.on_open = Box::new(f);
	}

	/// Bind to changing directory
	pub fn on_chdir<F: 'a>(&mut self, f: F)
	where
		F: Fn(&mut ::wtk::WindowTrait, &Path)
	{
		self.on_chdir = Box::new(f);
	}
}

impl<'a> ::wtk::Element for FileList<'a>
{
	fn render(&self, surface: ::wtk::surface::SurfaceView, force: bool) {
		self.list.render(surface, force);
	}
	fn resize(&self, w: u32, h: u32) {
		//self.list.resize(w, h);
	}
	fn handle_event(&self, ev: ::wtk::InputEvent, win: &mut ::wtk::WindowTrait) -> bool {
		self.list.handle_event(
			ev,
			|ent| {
				let item_name: &OsStr = &ent.name;
				let mut ps = self.cur_paths.borrow_mut();
				if item_name.as_bytes() == b".." {
					if ps.len() > 0 {
						ps.pop();
					}
				}
				else {
					ps.push( From::from(item_name) );
				}
				let path: ::std::ffi::OsString = {
					let mut v = Vec::new();
					for seg in ps.iter() {
						v.push(b'/');
						for &b in seg.as_bytes() {
							v.push(b);
						}
					}
					v.into()
					};
				let nh = match ::syscalls::vfs::Node::open(&*path)
					{
					Ok(v) => v,
					Err(e) => {
						kernel_log!("Error opening: {:?} - {:?}", item_name, e);
						return None;
						},
					};

				match nh.class()
				{
				::syscalls::vfs::NodeType::File => {
					(self.on_open)(win, Path::new(&*path));
					None
					},
				::syscalls::vfs::NodeType::Dir => {
					(self.on_chdir)(win, Path::new(&*path));
					Some( move || self.populate( &mut nh.into_dir().unwrap() ) )
					},
				::syscalls::vfs::NodeType::Symlink => {
					None
					},
				_ => None,
				}
				}
			)
	}
	fn with_element_at_pos(&self, pos: ::wtk::geom::PxPos, _dims: ::wtk::geom::PxDims, f: ::wtk::WithEleAtPosCb) -> bool {
		f(self, pos)
	}
}

struct FileEnt
{
	ty_str: &'static str,
	name: OsString,
	display_name: Option<String>,
}
impl FileEnt
{
	fn new(dir: &::syscalls::vfs::Dir, name: &[u8]) -> FileEnt {
		let node = dir.open_child(name);
		let node_ty = match node { Ok(n) => Some(n.class()), Err(_) => None };
		FileEnt {
			ty_str: match node_ty
				{
				Some(::syscalls::vfs::NodeType::File) => "f",
				Some(::syscalls::vfs::NodeType::Dir) => "d",
				Some(::syscalls::vfs::NodeType::Symlink) => "l",
				Some(::syscalls::vfs::NodeType::Special) => "s",
				None => "?",
				},
			name: OsString::from(name),
			display_name: if ::std::str::from_utf8(name).is_ok() {
					None
				}
				else {
					Some(String::from_utf8_lossy(name).into_owned())
				},
		}
	}
}
impl ::listview::Row for FileEnt {
	fn count(&self) -> usize {
		2
	}
	fn value(&self, col: usize) -> &str {
		match col
		{
		0 => self.ty_str,
		1 => if let Some(ref dn) = self.display_name {
				dn
			}
			else {
				self.name.to_str().unwrap()
			},
		_ => "",
		}
	}
}

