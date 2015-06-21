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

def_grp!( 0: GROUP_CORE = {
	0: CORE_LOGWRITE,
	1: CORE_EXITPROCESS,
	2: CORE_EXITTHREAD,
	3: CORE_STARTPROCESS,
	4: CORE_STARTTHREAD,
});

def_grp!( 1: GROUP_GUI = {
	0: GUI_NEWGROUP,
	1: GUI_NEWWINDOW,
});

def_grp!(2: GROUP_VFS = {
	0: VFS_OPEN,
});
