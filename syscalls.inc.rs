// Tifflin OS Project
// - By John Hodge (thePowersGang)
//
// syscalls.inc.rs
// - Common definition of system calls
//
// Included using #[path] from Kernel/Core/syscalls/mod.rs and Userland/libtifflin_syscalls/src/lib.rs

pub const GRP_OFS: usize = 16;

macro_rules! expand_expr { ($e:expr) => {$e}; }
macro_rules! def_grp {
	($val:tt: $name:ident = { $( $v:tt: $n:ident, )* }) => {
		pub const $name: u32 = expand_expr!($val);
		$( pub const $n: u32 = ($name << GRP_OFS) | expand_expr!($v); )*
	}
}
macro_rules! def_class {
	({ $( $v:tt: $n:ident, )* }) => {
		$( pub const $n: u16 = expand_expr!($v); )*
	}
}

def_grp!( 0: GROUP_CORE = {
	0: CORE_LOGWRITE,
	1: CORE_EXITPROCESS,
	2: CORE_EXITTHREAD,
	3: CORE_STARTPROCESS,
	4: CORE_STARTTHREAD,
	5: CORE_WAIT,
});

#[repr(C)]
#[derive(Debug)]
pub struct WaitItem {
	object: u32,
	flags: u32,
}

def_grp!( 1: GROUP_GUI = {
	0: GUI_NEWGROUP,
	1: GUI_NEWWINDOW,
});

def_grp!(2: GROUP_VFS = {
	0: VFS_OPENNODE,
	1: VFS_OPENFILE,
	2: VFS_OPENDIR,
});

def_grp!(3: GROUP_MEM = {
	0: MEM_ALLOCATE,
	1: MEM_REPROTECT,
	2: MEM_DEALLOCATE,
});

def_class!({
	0: VFS_FILE_READAT,
	1: VFS_FILE_WRITEAT,
	2: VFS_FILE_MEMMAP,
});
def_class!( {
	0: GUI_WIN_SHOWHIDE,
	1: GUI_WIN_REDRAW,
	1: GUI_WIN_BLITRECT,
});
