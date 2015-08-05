// Tifflin OS Project
// - By John Hodge (thePowersGang)
//
// syscalls.inc.rs
// - Common definition of system calls
//
// Included using #[path] from Kernel/Core/syscalls/mod.rs and Userland/libtifflin_syscalls/src/lib.rs

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

pub const TEXTINFO_KERNEL: usize = 0;

#[repr(C)]
#[derive(Debug)]
pub struct WaitItem {
	pub object: u32,
	pub flags: u32,
}

/// GUI System calls
def_grp!( 1: GROUP_GUI = {
	/// Create a new GUI group/session (requires capability, init only usually)
	=0: GUI_NEWGROUP,
	/// Set the passed group object to be the controlling group for this process
	=1: GUI_BINDGROUP,
	/// Create a new window in the current group
	=2: GUI_NEWWINDOW,
});

/// VFS Access
def_grp!(2: GROUP_VFS = {
	=0: VFS_OPENNODE,
	=1: VFS_OPENFILE,
	=2: VFS_OPENDIR,
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
	/// Opened file
	=2: CLASS_VFS_FILE = {
		/// Read data from the specified position in the file
		=0: VFS_FILE_READAT,
		/// Write to the specified position in the file
		=1: VFS_FILE_WRITEAT,
		/// Map part of the file into the current address space
		=2: VFS_FILE_MEMMAP,
	}|{
	},
	/// Opened directory
	=3: CLASS_VFS_DIR = {
		/// Read a selection of entries
		=0: VFS_DIR_READENTS,
	}|{
	},
	/// GUI Group/Session
	=4: CLASS_GUI_GROUP = {
		/// Force this group to be the active one (requires permission)
		=0: GUI_GRP_FORCEACTIVE,
	}|{
		/// Fires when the group is shown/hidden
		=0: EV_GUI_GRP_SHOWHIDE,
	},
	/// Window
	=5: CLASS_GUI_WIN = {
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
	}|{
		/// Fires when the input queue is non-empty
		=0: EV_GUI_WIN_INPUT,
	}
}

pub const GUI_WIN_FLAG_VISIBLE: u8 = 0;
pub const GUI_WIN_FLAG_MAXIMISED: u8 = 1;

#[derive(Debug)]
pub enum GuiEvent
{
	KeyUp(u32),
	KeyDown(u32),
	MouseMove(i32,i32),
	MouseUp(u8),
	MouseDown(u8),
}
impl ::core::convert::From<u64> for GuiEvent {
	fn from(v: u64) -> Self {
		assert!(v != !0);
		match v >> 48 {
		0 => GuiEvent::KeyUp  ( (v & 0xFFFFFFFF) as u32 ),
		1 => GuiEvent::KeyDown( (v & 0xFFFFFFFF) as u32 ),
		tag @ _ => panic!("Invalid tag value passed to GuiEvent::from {:#x}", tag),
		}
	}
}
impl ::core::convert::Into<u64> for GuiEvent {
	fn into(self) -> u64 {
		match self
		{
		GuiEvent::KeyUp  (ck) => (0 << 48) | (ck as u64),
		GuiEvent::KeyDown(ck) => (1 << 48) | (ck as u64),
		GuiEvent::MouseMove(dx,dy) => (2 << 48) | ((dx as u64) << 24) | ((dy as u64) << 0),
		GuiEvent::MouseUp  (btn) => (3 << 48) | (btn as u64),
		GuiEvent::MouseDown(btn) => (4 << 48) | (btn as u64),
		}
	}
}

