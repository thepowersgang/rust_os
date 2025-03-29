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
#![no_std]

extern crate key_codes;
pub use key_codes::KeyCode;

pub const GRP_OFS: usize = 16;


macro_rules! expand_expr { ($e:expr) => {$e}; }

// Define the non-object system calls (broken into groups)
macro_rules! def_groups {
	(
		$(
			$(#[$group_attrs:meta])*
			=$group_idx:tt: $group_name:ident = {
					$( $(#[$a:meta])* =$v:tt: $n:ident, )*
				}
		),*
		$(,)*
	) => {
		#[repr(u32)]
		#[allow(non_camel_case_types,dead_code)]
		enum Groups {
			$($group_name = expand_expr!($group_idx)),*
		}
		mod group_calls { $(
			#[repr(u32)]
			#[allow(non_camel_case_types,dead_code)]
			pub enum $group_name {
				$($n = expand_expr!($v),)*
			}
		)* }
		$( $(#[$group_attrs])* pub const $group_name: u32 = Groups::$group_name as u32; )*
		$( $( $(#[$a])* pub const $n: u32 = ($group_name << GRP_OFS) | (self::group_calls::$group_name::$n as u32); )* )*
		//pub const GROUP_NAMES: &'static [&'static str] = &[
		//	$(stringify!($group_name),)*
		//	]; 
		};
}

def_groups! {
	/// Core system calls, mostly thread management
	=0: GROUP_CORE = {
		/// Terminate the current process
		// NOTE: '0' is hard-coded in rustrt0/common.S
		=0: CORE_EXITPROCESS,
		/// Terminate the current thread
		=1: CORE_EXITTHREAD,
		/// Write a logging message
		=2: CORE_LOGWRITE,
		/// Write a hex value and string
		=3: CORE_DBGVALUE,
		/// Request a text string from the kernel
		=4: CORE_TEXTINFO,
		/// Start a new process (loader only, use loader API instead)
		=5: CORE_STARTPROCESS,
		/// Start a new thread in the current process
		=6: CORE_STARTTHREAD,
		/// Wait for any of a set of events
		=7: CORE_WAIT,
		/// Wait on a futex
		=8: CORE_FUTEX_SLEEP,
		/// Wake a number of sleepers on a futex
		=9: CORE_FUTEX_WAKE,
	},
	/// GUI System calls
	=1: GROUP_GUI = {
		/// Create a new GUI group/session (requires capability, init only usually)
		=0: GUI_NEWGROUP,
		/// Set the passed group object to be the controlling group for this process
		=1: GUI_BINDGROUP,
		/// Obtain a new handle to this window group
		=2: GUI_GETGROUP,
		/// Create a new window in the current group
		=3: GUI_NEWWINDOW,
	},
	/// Process memory management
	=2: GROUP_MEM = {
		=0: MEM_ALLOCATE,
		=1: MEM_REPROTECT,
		=2: MEM_DEALLOCATE,
	},
	/// Process memory management
	=3: GROUP_IPC = {
		/// Allocate a handle pair (returns two object handles)
		=0: IPC_NEWPAIR,
	},
	/// Netwokring
	=4: GROUP_NETWORK = {
		/// Connect a socket
		=0: NET_CONNECT,
		/// Start a socket server
		=1: NET_LISTEN,
		/// Open a free-form datagram 'socket'
		=2: NET_BIND,
	}
}

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


pub fn get_class_name(class_idx: u16) -> &'static str {
	CLASS_NAMES.get(class_idx as usize).unwrap_or(&"UNK")
}

pub const OBJECT_CLONE: u16 = 0x3FE;
pub const OBJECT_GETCLASS: u16 = 0x3FF;
pub const OBJECT_DROP: u16 = 0x7FF;

// Define all classes, using c-like enums to ensure that values are not duplicated
macro_rules! def_classes {
	(
		$(
			$(#[$class_attrs:meta])*
			=$class_idx:tt: $class_name:ident = {
					// By-reference (non-moving) methods
					$( $(#[$va:meta])* =$vv:tt: $vn:ident, )*
					--
					// By-value (moving) methods
					$( $(#[$ma:meta])* =$mv:tt: $mn:ident, )*
				}|{
					// Events
					$( $(#[$ea:meta])* =$ev:tt: $en:ident, )*
				}
		),*
		$(,)*
	) => {
		#[repr(u16)]
		#[allow(non_camel_case_types,dead_code)]
		enum Classes {
			$($class_name = expand_expr!($class_idx)),*
		}
		mod calls { $(
			//#[repr(u16)]
			#[allow(non_camel_case_types,dead_code)]
			pub enum $class_name {
				$($vn = expand_expr!($vv),)*
				$($mn = expand_expr!($mv)|0x400),*
			}
		)* }
		mod masks { $(
			#[allow(non_camel_case_types,dead_code)]
			pub enum $class_name { $($en = expand_expr!($ev)),* }
		)* }
		$( $(#[$class_attrs])* pub const $class_name: u16 = Classes::$class_name as u16; )*
		$( $( $(#[$va])* pub const $vn: u16 = self::calls::$class_name::$vn as u16; )* )*
		$( $( $(#[$ma])* pub const $mn: u16 = self::calls::$class_name::$mn as u16; )* )*
		$( $( $(#[$ea])* pub const $en: u32 = 1 << self::masks::$class_name::$en as usize; )* )*
		pub const CLASS_NAMES: &'static [&'static str] = &[
			$(stringify!($class_name),)*
			]; 
		};
}

// Object classes define the syscall interface followed by the object
def_classes! {
	/// Handle to a spawned process, used to communicate with it
	=0: CLASS_CORE_PROTOPROCESS = {
		/// Give the process one of this process's objects
		/// This method blocks if the child process hasn't popped the previous object
		=0: CORE_PROTOPROCESS_SENDOBJ,
		--
		/// Start the process executing
		=0: CORE_PROTOPROCESS_START,
	}|{
	},
	/// Handle to a spawned process, used to communicate with it
	=1: CLASS_CORE_PROCESS = {
		/// Request that the process be terminated
		=0: CORE_PROCESS_KILL,
		--
	}|{
		/// Wakes if the child process terminates
		=0: EV_PROCESS_TERMINATED,
	},
	/// A handle providing process inherent IPC
	=2: CLASS_CORE_THISPROCESS = {
		/// Receive a sent object
		=0: CORE_THISPROCESS_RECVOBJ,
		--
	}|{
	},
	/// Opened node
	=3: CLASS_VFS_NODE = {
		=0: VFS_NODE_GETTYPE,
		--
		=0: VFS_NODE_TOFILE,
		=1: VFS_NODE_TODIR,
		=2: VFS_NODE_TOLINK,
	}|{
	},
	/// Opened file
	=4: CLASS_VFS_FILE = {
		/// Get the size of the file (maximum addressable byte + 1)
		=0: VFS_FILE_GETSIZE,
		/// Read data from the specified position in the file
		=1: VFS_FILE_READAT,
		/// Write to the specified position in the file
		=2: VFS_FILE_WRITEAT,
		/// Map part of the file into the current address space
		=3: VFS_FILE_MEMMAP,
		--
	}|{
	},
	/// Opened directory
	=5: CLASS_VFS_DIR = {
		/// Create an enumerating handle
		=0: VFS_DIR_ENUMERATE,
		/// Open a child node
		=1: VFS_DIR_OPENCHILD,
		/// Open a sub-path
		=2: VFS_DIR_OPENPATH,
		--
	}|{
	},
	/// Enumerating directory
	=6: CLASS_VFS_DIRITER = {
		/// Read an entry
		=0: VFS_DIRITER_READENT,
		--
	}|{
	},
	/// Opened symbolic link
	=7: CLASS_VFS_LINK = {
		/// Read the destination path of the link
		=0: VFS_LINK_READ,
		--
	}|{
	},
	/// GUI Group/Session
	=8: CLASS_GUI_GROUP = {
		/// Force this group to be the active one (requires permission)
		=0: GUI_GRP_FORCEACTIVE,
		/// Get the count and extent of display surfaces
		/// Arguments: None
		/// Returns: Packed integers
		/// -  0..24(24): Total width
		/// - 24..48(24): Total height
		/// - 48..56( 8): Display count
		=1: GUI_GRP_TOTALOUTPUTS,
		/// Obtain the dimensions (and position) of an output
		/// Arguments:
		/// - Display index
		/// Returns: Packed integers
		/// -  0..16(16): Width
		/// - 16..32(16): Height
		/// - 32..48(16): X
		/// - 48..64(16): Y
		=2: GUI_GRP_GETDIMS,
		/// Get the intended viewport (i.e. ignoring global toolbars)
		/// Returns: Packed integers
		/// -  0..16(16): Width
		/// - 16..32(16): Height
		/// - 32..48(16): RelX
		/// - 48..64(16): RelY
		=3: GUI_GRP_GETVIEWPORT,
		--
	}|{
		/// Fires when the group is shown/hidden
		=0: EV_GUI_GRP_SHOWHIDE,
	},
	/// Window
	=9: CLASS_GUI_WIN = {
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
		--
	}|{
		/// Fires when the input queue is non-empty
		=0: EV_GUI_WIN_INPUT,
		///// Fires when focus is lost/gained
		//=1: EV_GUI_WIN_FOCUS,
	},

	/// Remote procedure call channel
	=10: CLASS_IPC_RPC = {
		/// Send a message over the channel (RpcMessage, limited size)
		=0: IPC_RPC_SEND,
		/// Receive a message
		=1: IPC_RPC_RECV,
	--
	}|{
		/// Fires when the channel has a message waiting
		=0: EV_IPC_RPC_RECV,
	},

	/// Socket server
	=11: CLASS_SERVER = {
		/// Check for a new client
		=0: NET_SERVER_ACCEPT,
	--
	}|{
	},
	/// Socket connection
	=12: CLASS_SOCKET = {
		/// Read data
		=0: NET_CONNSOCK_RECV,
		/// Send data
		=1: NET_CONNSOCK_SEND,
		///
		=2: NET_CONNSOCK_SHUTDOWN,
	--
	}|{
		/// Event raised when there is data ready to read
		=0: EV_NET_CONNSOCK_RECV,
	},
	/// Free-bind socket
	=13: CLASS_FREESOCKET = {
		/// Receive a packet (if available)
		=0: NET_FREESOCK_RECVFROM,
		/// Send a packet to an address
		=1: NET_FREESOCK_SENDTO,
	--
	}|{
	},
/*
	/// A registered read/write buffer
	=12: CLASS_BUFFER = {
		--
		/// Release the buffer (and return the memory to userland)
		=0: BUFFER_RELEASE,
	}|{
		/// Fires when the buffer is used (populated/read)
		=0: EV_BUFFER_CONSUMED,
	}
*/
}


macro_rules! enum_to_from {
	($enm:ident => $ty:ty : $( $(#[$a:meta])* $n:ident = $v:expr,)*) => {
		#[derive(Debug)]
		pub enum $enm
		{
			$( $(#[$a])* $n = $v,)*
		}
		//impl ::core::convert::From<$ty> for ::core::option::Option<$enm> {
		//	fn from(v: $ty) -> Self {
		//		match v
		//		{
		//		$($v => Some($enm::$n),)*
		//		_ => None,
		//		}
		//	}
		//}
		impl $enm {
			#[allow(dead_code)]
			pub fn try_from(v: $ty) -> Result<Self,$ty> {
				match v
				{
				$($v => Ok($enm::$n),)*
				// TODO: This should not panic - it should return Result/Option instead
				_ => Err(v),
				}
			}
		}
		impl ::core::convert::Into<$ty> for $enm {
			fn into(self) -> $ty {
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


enum_to_from!{ GuiWinFlag => u8:
	Visible = 0,
	Maximised = 1,
}

//include!("keycodes.inc.rs");

/// Fixed-capacity string buffer (6 bytes)
#[derive(Copy,Clone)]
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
		assert!(v.len() <= 6);
		rv[..v.len()].clone_from_slice(v);
		FixedStr6(rv)
	}
}
impl ::core::convert::From<[u8; 6]> for FixedStr6 {
	fn from(v: [u8; 6]) -> FixedStr6 {
		FixedStr6(v)
	}
}
/// Fixed-capacity string buffer (8 bytes)
#[derive(Copy,Clone)]
pub struct FixedStr8([u8; 8]);
impl ::core::fmt::Debug for FixedStr8 {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		::core::fmt::Debug::fmt(&**self, f)
	}
}
impl ::core::ops::Deref for FixedStr8 {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str { ::core::str::from_utf8(&self.0).expect("Invalid UTF-8 from kernel").split('\0').next().unwrap() }
}
impl<'a> ::core::convert::From<&'a str> for FixedStr8 {
	fn from(v: &str) -> FixedStr8 { From::from(v.as_bytes()) }
}
impl<'a> ::core::convert::From<&'a [u8]> for FixedStr8 {
	fn from(v: &[u8]) -> FixedStr8 {
		let mut rv = [0; 8];
		assert!(v.len() <= 8);
		rv[..v.len()].clone_from_slice(v);
		FixedStr8(rv)
	}
}
impl ::core::convert::From<[u8; 8]> for FixedStr8 {
	fn from(v: [u8; 8]) -> FixedStr8 {
		FixedStr8(v)
	}
}
impl ::core::convert::From<u64> for FixedStr8 {
	fn from(v: u64) -> FixedStr8 {
		// SAFE: POD
		FixedStr8( unsafe { ::core::mem::transmute(v) } )
	}
}
impl ::core::convert::From<FixedStr8> for u64 {
	fn from(v: FixedStr8) -> u64 {
		// SAFE: POD
		unsafe { ::core::mem::transmute(v.0) }
	}
}

#[derive(Copy,Clone,Debug)]
/// GUI Window event
pub enum GuiEvent
{
	#[allow(dead_code)]
	/// Placeholder empty event
	None,
	
	/// Window size changed
	Resize,

	/// Key released
	KeyUp(KeyCode),
	/// Key pressed
	KeyDown(KeyCode),
	/// Key fired (pressed+released with no intermediate keys)
	KeyFire(KeyCode),
	/// Translated text from a keypress
	Text(FixedStr6),
	
	/// Mouse movement event - X,Y, dX, dY
	MouseMove(u32,u32, i16,i16),
	/// Mouse button released - X,Y, Button
	MouseUp(u32,u32, u8),
	/// Mouse button pressed - X,Y, Button
	MouseDown(u32,u32, u8),
	/// Mouse button clicked (pressed+released with minimal movement and elapsed time)
	MouseClick(u32,u32, u8),
	/// Mouse button double-clicked (clicked twice within timeout)
	MouseDblClick(u32,u32, u8),
	/// Triple-clicked
	MouseTriClick(u32,u32, u8),
}

pub type RpcMessage = [u8; 32];

// --------------------------------------------------------------------
// Network
// --------------------------------------------------------------------
enum_to_from!{ SocketError => u32:
	/// No data waiting/avaliable
	NoData = 0,
	/// An invalid value was passed to the call
	InvalidValue = 1,
	/// The specified address was already in use
	AlreadyInUse = 2,
}
enum_to_from!{ SocketShutdownSide => u8:
	Transmit = 0,
	Receive = 1,
}
// Values for the `addr_ty` field of SocketAddress
enum_to_from!{ SocketAddressType => u8:
	/// Ethernet II MAC addresses, only supports 'SocketPortType::Raw'
	Mac = 0,
	/// IPv4 addresses
	Ipv4 = 1,
	/// IPv6 addresses
	Ipv6 = 2,
}
enum_to_from!{ SocketPortType => u8:
	/// Raw frames
	Raw = 0,
	/// Transmission Control Protocol
	Tcp = 1,
	/// User Datagram Protocol
	Udp = 2,
	/// Stream Control Transmission Protocol
	Sctp = 3,
}
#[derive(Default,Copy,Clone,Debug)]
#[repr(C)]
pub struct SocketAddress
{
	pub port_ty: u8,
	pub addr_ty: u8,
	pub port: u16,
	pub addr: [u8; 16],
}
#[derive(Default,Copy,Clone)]
#[repr(C)]
pub struct MaskedSocketAddress
{
	pub addr: SocketAddress,
	pub mask: u8,
}

