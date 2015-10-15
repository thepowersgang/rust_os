// Tifflin OS Project
// - By John Hodge (thePowersGang)
//
// syscalls.inc.rs
// - Common definition of system calls
//
// Included using #[path] from Kernel/Core/syscalls/mod.rs and Userland/libtifflin_syscalls/src/lib.rs
//! System call IDs and user-kernel interface types
//! 
//! There are two broad types of system calls: free calls and object calls.
//! 
//! Free calls either construct a new object instance, or directly manipulate/query state.

pub const GRP_OFS: usize = 16;


macro_rules! expand_expr { ($e:expr) => {$e}; }

// Define a group of system calls
// TODO: Restructure like the class list
macro_rules! def_grp {
	($val:tt: $name:ident = { $( $(#[$a:meta])* =$v:tt: $n:ident, )* }) => {
		pub const $name: u32 = expand_expr!($val);
		$( $(#[$a])* pub const $n: u32 = ($name << GRP_OFS) | expand_expr!($v); )*
	}
}

/// Core system calls, mostly thread management
def_grp!( 0: GROUP_CORE = {
	/// Write a logging message
	=0: CORE_LOGWRITE,
	/// Request a text string from the kernel
	=1: CORE_TEXTINFO,
	/// Terminate the current process
	// NOTE: '2' is hard-coded in rustrt0
	=2: CORE_EXITPROCESS,
	/// Terminate the current thread
	=3: CORE_EXITTHREAD,
	/// Start a new process (loader only, use loader API instead)
	=4: CORE_STARTPROCESS,
	/// Start a new thread in the current process
	=5: CORE_STARTTHREAD,
	/// Wait for any of a set of events
	=6: CORE_WAIT,
});

/// Value for `get_text_info`'s `unit` argument, indicating kernel core
pub const TEXTINFO_KERNEL: u32 = 0;

#[repr(C)]
#[derive(Debug)]
/// Object reference used by the CORE_WAIT system call
pub struct WaitItem {
	/// Object ID
	pub object: u32,
	/// Class-specific wait flags
	pub flags: u32,
}

/// GUI System calls
def_grp!( 1: GROUP_GUI = {
	/// Create a new GUI group/session (requires capability, init only usually)
	=0: GUI_NEWGROUP,
	/// Set the passed group object to be the controlling group for this process
	=1: GUI_BINDGROUP,
	/// Obtain a new handle to this window group
	=2: GUI_GETGROUP,
	/// Create a new window in the current group
	=3: GUI_NEWWINDOW,
});

/// VFS Access
def_grp!(2: GROUP_VFS = {
	=0: VFS_OPENNODE,
	=1: VFS_OPENFILE,
	=2: VFS_OPENDIR,
	=3: VFS_OPENLINK,
});

/// Process memory management
def_grp!(3: GROUP_MEM = {
	=0: MEM_ALLOCATE,
	=1: MEM_REPROTECT,
	=2: MEM_DEALLOCATE,
});


// Define all classes, using c-like enums to ensure that values are not duplicated
macro_rules! def_classes {
	( $($(#[$ca:meta])* =$cval:tt: $cname:ident = { $( $(#[$a:meta])* =$v:tt: $n:ident, )* }|{ $( $(#[$ea:meta])* =$ev:tt: $en:ident, )* }),* ) => {
		#[repr(u16)]
		#[allow(non_camel_case_types,dead_code)]
		enum Classes { $($cname = expand_expr!($cval)),* }
		mod calls { $(
			//#[repr(u16)]
			#[allow(non_camel_case_types,dead_code)]
			pub enum $cname { $($n = expand_expr!($v)),* }
		)* }
		mod masks { $(
			#[allow(non_camel_case_types,dead_code)]
			pub enum $cname { $($en = expand_expr!($ev)),* }
		)* }
		$( $(#[$ca])* pub const $cname: u16 = Classes::$cname as u16; )*
		$( $( $(#[$a])* pub const $n: u16 = self::calls::$cname::$n as u16; )* )*
		$( $( $(#[$ea])* pub const $en: u32 = 1 << self::masks::$cname::$en as usize; )* )*
		};
}

def_classes! {
	/// Handle to a spawned process, used to communicate with it
	=0: CLASS_CORE_PROCESS = {
		/// Request that the process be terminated
		=0: CORE_PROCESS_KILL,
		/// Give the process one of this process's objects
		=1: CORE_PROCESS_SENDOBJ,
		/// Send a message to the object
		=2: CORE_PROCESS_SENDMSG,
	}|{
		=0: EV_PROCESS_TERMINATED,
	},
	=1: CLASS_CORE_THISPROCESS = {
		/// Fetch a handle to the 'n'th object of the specified class that hasn't been claimed
		=0: CORE_THISPROCESS_RECVOBJ,
		=1: CORE_THISPROCESS_RECVMSG,
	}|{
		=0: EV_THISPROCESS_RECVOBJ,
		=1: EV_THISPROCESS_RECVMSG,
	},
	/// Opened node
	=2: CLASS_VFS_NODE = {
		=0: VFS_NODE_GETTYPE,
		=1: VFS_NODE_TOFILE,
		=2: VFS_NODE_TODIR,
		=3: VFS_NODE_TOLINK,
	}|{
	},
	/// Opened file
	=3: CLASS_VFS_FILE = {
		/// Get the size of the file (maximum addressable byte + 1)
		=0: VFS_FILE_GETSIZE,
		/// Read data from the specified position in the file
		=1: VFS_FILE_READAT,
		/// Write to the specified position in the file
		=2: VFS_FILE_WRITEAT,
		/// Map part of the file into the current address space
		=3: VFS_FILE_MEMMAP,
	}|{
	},
	/// Opened directory
	=4: CLASS_VFS_DIR = {
		/// Read an entry
		=0: VFS_DIR_READENT,
	}|{
	},
	/// Opened symbolic link
	=5: CLASS_VFS_LINK = {
		=0: VFS_LINK_READ,
	}|{
	},
	/// GUI Group/Session
	=6: CLASS_GUI_GROUP = {
		/// Force this group to be the active one (requires permission)
		=0: GUI_GRP_FORCEACTIVE,
	}|{
		/// Fires when the group is shown/hidden
		=0: EV_GUI_GRP_SHOWHIDE,
	},
	/// Window
	=7: CLASS_GUI_WIN = {
		/// Set the show/hide state of the window
		=0: GUI_WIN_SETFLAG,
		/// Trigger a redraw of the window
		=1: GUI_WIN_REDRAW,
		/// Copy data from this process into the window
		=2: GUI_WIN_BLITRECT,
		/// Fill a region of the window with the specified colour
		=3: GUI_WIN_FILLRECT,
		/// Read an event from the queue. 64-bit return value, !0 = none, otherwise 16/48 tag and data
		// TODO: Pass a &mut GuiEvent instead of deserialsiging a u64
		=4: GUI_WIN_GETEVENT,
		/// Obtain the window dimensions
		=5: GUI_WIN_GETDIMS,
		/// Set window dimensions (may be restricted)
		=6: GUI_WIN_SETDIMS,
		/// Obtain window position
		=7: GUI_WIN_GETPOS,
		/// Set window position (will be clipped to visible area)
		=8: GUI_WIN_SETPOS,
		/// Set the client region rectangle
		=9: GUI_WIN_SETCLIENTREGION,
	}|{
		/// Fires when the input queue is non-empty
		=0: EV_GUI_WIN_INPUT,
	}
}

pub const GUI_WIN_FLAG_VISIBLE: u8 = 0;
pub const GUI_WIN_FLAG_MAXIMISED: u8 = 1;

macro_rules! enum_to_from {
	($enm:ident => $ty:ty : $( $(#[$a:meta])* $n:ident = $v:expr,)*) => {
		#[derive(Debug)]
		pub enum $enm
		{
			$( $($a)* $n,)*
		}
		impl ::core::convert::From<$ty> for $enm {
			fn from(v: $ty) -> Self {
				match v
				{
				$($v => $enm::$n,)*
				_ => panic!("Unknown value for {} - {}", stringify!($enm), v),
				}
			}
		}
		impl ::core::convert::Into<$ty> for $enm {
			fn into(self) -> $ty
			{
				match self
				{
				$($enm::$n => $v,)*
				}
			}
		}
	}
}

enum_to_from!{ VFSError => u32:
	FileNotFound = 0,
	TypeError = 1,
	PermissionDenied = 2,
	FileLocked = 3,
	MalformedPath = 4,
}
enum_to_from!{ VFSNodeType => u32:
	File = 0,
	Dir = 1,
	Symlink = 2,
	Special = 3,
}
enum_to_from!{ VFSFileOpenMode => u8:
	ReadOnly = 1,
	Execute  = 2,
	ExclRW   = 3,
	UniqueRW = 4,
	Append   = 5,
	Unsynch  = 6,
}

enum_to_from!{ VFSMemoryMapMode => u8:
	// /// Read-only mapping of a file
	ReadOnly = 0,
	// /// Executable mapping of a file
	Execute = 1,
	// /// Copy-on-write (used for executable files)
	COW = 2,
	// /// Allows writing to the backing file
	WriteBack = 3,
}

include!("keycodes.inc.rs");

/// Fixed-capacity string buffer (6 bytes)
pub struct FixedStr6([u8; 6]);
impl ::core::fmt::Debug for FixedStr6 {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		::core::fmt::Debug::fmt(&**self, f)
	}
}
impl ::core::ops::Deref for FixedStr6 {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str { ::core::str::from_utf8(&self.0).expect("Invalid UTF-8 from kernel").split('\0').next().unwrap() }
}
impl<'a> ::core::convert::From<&'a str> for FixedStr6 {
	fn from(v: &str) -> FixedStr6 { From::from(v.as_bytes()) }
}
impl<'a> ::core::convert::From<&'a [u8]> for FixedStr6 {
	fn from(v: &[u8]) -> FixedStr6 {
		let mut rv = [0; 6];
		rv.clone_from_slice(v);
		FixedStr6(rv)
	}
}
impl ::core::convert::From<[u8; 6]> for FixedStr6 {
	fn from(v: [u8; 6]) -> FixedStr6 {
		FixedStr6(v)
	}
}

#[derive(Debug)]
/// GUI Window event
pub enum GuiEvent
{
	#[allow(dead_code)]
	/// Placeholder empty event
	None,
	/// Key released
	KeyUp(KeyCode),
	/// Key pressed
	KeyDown(KeyCode),
	/// Translated text from a keypress
	Text(FixedStr6),
	/// Mouse movement event - X,Y, dX, dY
	MouseMove(u32,u32, i16,i16),
	/// Mouse button released - X,Y, Button
	MouseUp(u32,u32, u8),
	/// Mouse button pressed - X,Y, Button
	MouseDown(u32,u32, u8),
}

