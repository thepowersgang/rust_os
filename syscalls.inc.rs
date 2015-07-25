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

def_grp!( 0: GROUP_CORE = {
	/// Write a logging message
	=0: CORE_LOGWRITE,
	=1: CORE_EXITPROCESS,
	=2: CORE_EXITTHREAD,
	=3: CORE_STARTPROCESS,
	=4: CORE_STARTTHREAD,
	=5: CORE_WAIT,
	=6: CORE_SENDMSG,
	=7: CORE_RECVMSG,
});

#[repr(C)]
#[derive(Debug)]
pub struct WaitItem {
	pub object: u32,
	pub flags: u32,
}

def_grp!( 1: GROUP_GUI = {
	/// Create a new GUI group/session (requires capability, init only usually)
	=0: GUI_NEWGROUP,
	/// Set the passed group object to be the controlling group for this process
	=1: GUI_BINDGROUP,
	/// Create a new window in the current group
	=2: GUI_NEWWINDOW,
});

def_grp!(2: GROUP_VFS = {
	=0: VFS_OPENNODE,
	=1: VFS_OPENFILE,
	=2: VFS_OPENDIR,
});

def_grp!(3: GROUP_MEM = {
	=0: MEM_ALLOCATE,
	=1: MEM_REPROTECT,
	=2: MEM_DEALLOCATE,
});


// Define all classes, using c-like enums to ensure that values are not duplicated
macro_rules! def_classes {
	( $($(#[$ca:meta])* =$cval:tt: $cname:ident = { $( $(#[$a:meta])* =$v:tt: $n:ident, )* }|{ $( $(#[$ea:meta])* =$ev:tt: $en:ident, )* }),* ) => {
		#[repr(u16)]
		enum Classes { $($cname = expand_expr!($cval)),* }
		mod calls { $(
			//#[repr(u16)]
			pub enum $cname { $($n = expand_expr!($v)),* }
		)* }
		mod masks { $(
			pub enum $cname { $($en = expand_expr!($ev)),* }
		)* }
		$( $(#[$ca])* pub const $cname: u16 = Classes::$cname as u16; )*
		$( $( $(#[$a])* pub const $n: u16 = self::calls::$cname::$n as u16; )* )*
		$( $( $(#[$ea])* pub const $en: u32 = 1 << self::masks::$cname::$en as usize; )* )*
		};
}

def_classes! {
	/// Handle to a spawned process, used to communicate with it
	=0: CLASS_PROCESS = {
		=0: CORE_PROCESS_KILL,
		=1: CORE_PROCESS_SENDOBJ,
		=2: CORE_PROCESS_SENDMSG,
	}|{
	},
	/// Opened file
	=1: CLASS_VFS_FILE = {
		=0: VFS_FILE_READAT,
		=1: VFS_FILE_WRITEAT,
		=2: VFS_FILE_MEMMAP,
	}|{
	},
	/// GUI Group/Session
	=2: CLASS_GUI_GROUP = {
		=0: GUI_GRP_FORCEACTIVE,
	}|{
		=0: EV_GUI_GRP_SHOWHIDE,
	},
	/// Window
	=3: CLASS_GUI_WIN = {
		=0: GUI_WIN_SHOWHIDE,
		=1: GUI_WIN_REDRAW,
		=2: GUI_WIN_BLITRECT,
		=3: GUI_WIN_FILLRECT,
	}|{
		=0: EV_GUI_WIN_INPUT,
	}
}
