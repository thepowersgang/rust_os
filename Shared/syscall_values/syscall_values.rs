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
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]	// for `generic_const_exprs`

extern crate key_codes;
pub use key_codes::KeyCode;

#[macro_use]
mod macros;

mod traits;
pub use self::traits::{Args,ToUsizeArray};

mod fixed_string;
pub use self::fixed_string::{FixedStr6,FixedStr8};

pub const GRP_OFS: usize = 16;

def_groups! {
	/// Core system calls, mostly thread management
	=0: GROUP_CORE = {
		/// Terminate the current process
		// NOTE: The ID '0' is hard-coded in rustrt0/common.S
		=0: CORE_EXITPROCESS(status: u32),
		/// Terminate the current thread
		=1: CORE_EXITTHREAD(),
		/// Write a logging message
		=2: CORE_LOGWRITE<'a>(msg: &'a [u8]),
		/// Write a hex value and string
		=3: CORE_DBGVALUE<'a>(msg: &'a [u8], value: usize),
		/// Request a text string from the kernel
		=4: CORE_TEXTINFO<'a>(group: u32, value: u32, dst: &'a mut [u8]),
		? not(arch="native")
		/// Start a new process (for use by the loader only, use loader API instead)
		=5: CORE_STARTPROCESS<'a>(name: &'a str, clone_start: usize, clone_end: usize),
		? arch="native"
		/// Start a new process (loader only, use loader API instead)
		=5: CORE_STARTPROCESS<'a>(handle: u32, name: &'a str, args_nul: &'a [u8]),
		/// Start a new thread in the current process
		=6: CORE_STARTTHREAD(ip: usize, sp: usize, tls_base: usize),
		/// Wait for any of a set of events
		/// 
		/// - If `wake_time_monotonic` is zero, the call just polls
		/// - If `wake_time_monotonic` is `!0` then no timeout is applied
		/// - Otherwise, the call will wake when the system ticks value passes this value
		/// 
		/// Returns the number of items that were ready
		=7: CORE_WAIT<'a>(items: &'a mut [WaitItem], wake_time_monotonic: u64) -> u32,
		/// Wait on a futex
		=8: CORE_FUTEX_SLEEP<'a>(addr: &'a ::core::sync::atomic::AtomicUsize, sleep_if_val: usize),
		/// Wake a number of sleepers on a futex
		=9: CORE_FUTEX_WAKE<'a>(addr: &'a ::core::sync::atomic::AtomicUsize, num_to_wake: usize),
		/// Get current system time in ticks
		=10: CORE_SYSTEM_TICKS(),// -> u64,
	},
	/// GUI System calls
	=1: GROUP_GUI = {
		/// Create a new GUI group/session (requires capability, init only usually)
		=0: GUI_NEWGROUP<'a>(name: &'a str),
		/// Set the passed group object to be the controlling group for this process
		=1: GUI_BINDGROUP(obj: u32),
		/// Obtain a new handle to this window group
		=2: GUI_GETGROUP(),
		/// Create a new window in the current group
		=3: GUI_NEWWINDOW<'a>(name: &'a str),
	},
	/// Process memory management
	=2: GROUP_MEM = {
		=0: MEM_ALLOCATE(addr: usize, count: usize),
		=1: MEM_REPROTECT(addr: usize, protection: u8),
		=2: MEM_DEALLOCATE(addr: usize),
	},
	/// Process memory management
	=3: GROUP_IPC = {
		/// Allocate a handle pair (returns two object handles)
		=0: IPC_NEWPAIR(),
	},
	/// Netwokring
	=4: GROUP_NETWORK = {
		/// Connect a socket
		=0: NET_CONNECT<'a>(addr: &'a SocketAddress),
		/// Start a socket server
		=1: NET_LISTEN<'a>(addr: &'a SocketAddress),
		/// Open a free-form datagram 'socket'
		=2: NET_BIND<'a>(local: &'a SocketAddress, remote: &'a MaskedSocketAddress),

		/// Get the details of a network interface
		/// 
		/// Returns 0 on success, 1 when the index references an empty slot, and !0 when the end of the list is reached
		=4: NET_ENUM_INTERFACES<'a>(index: usize, data: &'a mut NetworkInterface) -> Option<bool>,

		/// Obtain an interafce address by index, address type is specified in `data.addr_ty`
		/// 
		/// Returns:
		/// - `None` when index is too large
		/// - `Some(true)` when `data` is populated
		/// - `Some(false)` when the index points to a non-poulated entry
		=5: NET_ENUM_ADDRESS<'a>(index: usize, data: &'a mut NetworkAddress) -> Option<bool>,

		/// Obtain a route by index, route type is specified in `data.addr_ty`
		/// 
		/// Returns:
		/// - `None` when index is too large
		/// - `Some(true)` when `data` is populated
		/// - `Some(false)` when the index points to a non-poulated entry
		=6: NET_ENUM_ROUTE<'a>(index: usize, data: &'a mut NetworkRoute) -> Option<bool>,
	}
}

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

// Object classes define the syscall interface followed by the object
def_classes! {
	/// Handle to a spawned process, used to communicate with it
	=0: CLASS_CORE_PROTOPROCESS = {
		/// Give the process one of this process's objects
		/// This method blocks if the child process hasn't popped the previous object
		=0: CORE_PROTOPROCESS_SENDOBJ(tag: FixedStr8, object_handle: u32),
		--
		/// Start the process executing
		=0: CORE_PROTOPROCESS_START(ip: usize, sp: usize) -> CLASS_CORE_PROCESS,
	}|{
	},
	/// Handle to a spawned process, used to communicate with it
	=1: CLASS_CORE_PROCESS = {
		/// Request that the process be terminated
		=0: CORE_PROCESS_KILL(),
		--
	}|{
		/// Wakes if the child process terminates
		=0: EV_PROCESS_TERMINATED,
	},
	/// A handle providing process inherent IPC
	=2: CLASS_CORE_THISPROCESS = {
		/// Receive a sent object
		=0: CORE_THISPROCESS_RECVOBJ(tag: FixedStr8, class: u16) -> u32,
		--
	}|{
	},
	/// Opened node
	=3: CLASS_VFS_NODE = {
		=0: VFS_NODE_GETTYPE() -> VFSNodeType,
		--
		=0: VFS_NODE_TOFILE(mode: VFSFileOpenMode) -> CLASS_VFS_FILE,
		=1: VFS_NODE_TODIR() -> CLASS_VFS_DIR,
		=2: VFS_NODE_TOLINK() -> CLASS_VFS_LINK,
	}|{
	},
	/// Opened file
	=4: CLASS_VFS_FILE = {
		/// Get the size of the file (maximum addressable byte + 1)
		=0: VFS_FILE_GETSIZE() -> u64,
		/// Read data from the specified position in the file
		=1: VFS_FILE_READAT<'a>(ofs: u64, data: &'a mut [u8]),
		/// Write to the specified position in the file
		=2: VFS_FILE_WRITEAT<'a>(ofs: u64, data: &'a [u8]),
		/// Map part of the file into the current address space
		=3: VFS_FILE_MEMMAP(ofs: u64, size: usize, addr: usize, mode: VFSMemoryMapMode),
		--
	}|{
	},
	/// Opened directory
	=5: CLASS_VFS_DIR = {
		/// Create an enumerating handle
		=0: VFS_DIR_ENUMERATE() -> CLASS_VFS_DIRITER,
		/// Open a child node
		=1: VFS_DIR_OPENCHILD<'a>(name: &'a [u8]) -> Result<CLASS_VFS_NODE, VFSError>,
		/// Open a sub-path
		=2: VFS_DIR_OPENPATH<'a>(path: &'a [u8]) -> Result<CLASS_VFS_NODE, VFSError>,
		--
	}|{
	},
	/// Enumerating directory
	=6: CLASS_VFS_DIRITER = {
		/// Read an entry, and return the length of the name (zero when the end is reached)
		=0: VFS_DIRITER_READENT<'a>(name: &'a mut [u8]) -> usize,
		--
	}|{
	},
	/// Opened symbolic link
	=7: CLASS_VFS_LINK = {
		/// Read the destination path of the link, returns the length of the link
		=0: VFS_LINK_READ<'a>(buf: &'a mut [u8]) -> usize,
		--
	}|{
	},
	/// GUI Group/Session
	=8: CLASS_GUI_GROUP = {
		/// Force this group to be the active one (requires permission)
		=0: GUI_GRP_FORCEACTIVE(),
		/// Get the count and extent of display surfaces
		/// Arguments: None
		/// Returns: Packed integers
		/// -  0..24(24): Total width
		/// - 24..48(24): Total height
		/// - 48..56( 8): Display count
		=1: GUI_GRP_TOTALOUTPUTS() -> u64,
		/// Obtain the dimensions (and position) of an output
		/// Arguments:
		/// - Display index
		/// Returns: Packed integers
		/// -  0..16(16): Width
		/// - 16..32(16): Height
		/// - 32..48(16): X
		/// - 48..64(16): Y
		=2: GUI_GRP_GETDIMS(index: usize) -> u64,
		/// Get the intended viewport (i.e. ignoring global toolbars)
		/// Returns: Packed integers
		/// -  0..16(16): Width
		/// - 16..32(16): Height
		/// - 32..48(16): RelX
		/// - 48..64(16): RelY
		=3: GUI_GRP_GETVIEWPORT(index: usize) -> u64,
		--
	}|{
		/// Fires when the group is shown/hidden
		=0: EV_GUI_GRP_SHOWHIDE,
	},
	/// Window
	=9: CLASS_GUI_WIN = {
		/// Set the show/hide state of the window
		=0: GUI_WIN_SETFLAG(flag: GuiWinFlag, is_on: bool),
		/// Trigger a redraw of the window
		=1: GUI_WIN_REDRAW(),
		/// Copy data from this process into the window
		=2: GUI_WIN_BLITRECT<'a>(x: u32, y: u32, w: u32, data: &'a [u32], stride: usize),
		/// Fill a region of the window with the specified colour
		=3: GUI_WIN_FILLRECT(x: u32, y: u32, w: u32, h: u32, colour: u32),
		/// Read an event from the queue. 64-bit return value, !0 = none, otherwise 16/48 tag and data
		=4: GUI_WIN_GETEVENT<'a>(event: &'a mut GuiEvent),
		/// Obtain the window dimensions
		=5: GUI_WIN_GETDIMS(),
		/// Set window dimensions (may be restricted)
		=6: GUI_WIN_SETDIMS(w: u32, h: u32),
		/// Obtain window position
		=7: GUI_WIN_GETPOS(),
		/// Set window position (will be clipped to visible area)
		=8: GUI_WIN_SETPOS(x: u32, y: u32),
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
		=0: IPC_RPC_SEND<'a>(msg: &'a RpcMessage, obj: u32),
		/// Receive a message
		=1: IPC_RPC_RECV<'a>(msg: &'a mut RpcMessage),
	--
	}|{
		/// Fires when the channel has a message waiting
		=0: EV_IPC_RPC_RECV,
	},

	// --- Networking ---

	/// Socket server
	=11: CLASS_SERVER = {
		/// Check for a new client
		=0: NET_SERVER_ACCEPT<'a>(out_addr: &'a mut SocketAddress) -> CLASS_SOCKET,
	--
	}|{
	},
	/// Socket connection
	=12: CLASS_SOCKET = {
		/// Read data
		=0: NET_CONNSOCK_RECV<'a>(data: &'a mut [u8]),
		/// Send data
		=1: NET_CONNSOCK_SEND<'a>(data: &'a [u8]),
		///
		=2: NET_CONNSOCK_SHUTDOWN(side: SocketShutdownSide),
	--
	}|{
		/// Event raised when there is data ready to read
		=0: EV_NET_CONNSOCK_RECV,
	},
	/// Free-bind socket
	=13: CLASS_FREESOCKET = {
		/// Receive a packet (if available)
		=0: NET_FREESOCK_RECVFROM<'a>(data: &'a mut [u8], addr: &'a mut SocketAddress),
		/// Send a packet to an address
		=1: NET_FREESOCK_SENDTO<'a>(data: &'a [u8], addr: &'a SocketAddress),
	--
	}|{
		/// Event raised when there is data ready to read
		=0: EV_NET_FREESOCK_RECV,
	},
	/// Network management functions
	=14: CLASS_NET_MANAGEMENT = {
		/// Add a new address to an interface
		=0: NET_MGMT_ADD_ADDRESS<'a>(index: usize, addr: &'a NetworkAddress, subnet_len: u8) -> Result<(),()>,
		/// Remove an address from an interface
		=1: NET_MGMT_DEL_ADDRESS<'a>(index: usize, addr: &'a NetworkAddress, subnet_len: u8) -> Result<(),()>,

		/// Add a new route
		=2: NET_MGMT_ADD_ROUTE<'a>(data: &'a NetworkRoute),
		/// Delete a route
		=3: NET_MGMT_DEL_ROUTE<'a>(data: &'a NetworkRoute),
		--
	}|{
		=0: EV_NET_MGMT_INTERFACE,
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

enum_to_from!{
	/// Unit/group names for the [CORE_TEXTINFO] call
	TextInfo => u32 :
		/// Kernel core
		Kernel = 0,
		/// Network stack
		Network = 1,
}
enum_to_from!{
	TextInfoKernel => u32 :
		Version = 0,
		BuildString = 1,
}
enum_to_from!{
	TextInfoNetwork => u32 :
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
	/// No route to host
	NoRoute = 3,
	SocketClosed = 4,
	ConnectionReset = 5,
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
/// A socket address with number of valid bits (used for restricting the range of free socket listens)
pub struct MaskedSocketAddress
{
	pub addr: SocketAddress,
	pub mask: u8,
}
#[derive(Default,Copy,Clone)]
#[derive(PartialEq)]
#[repr(C)]
pub struct NetworkInterface
{
	pub mac_addr: [u8; 6],
}
#[derive(Default,Copy,Clone)]
#[repr(C)]
pub struct NetworkAddress
{
	pub addr_ty: u8,
	pub addr: [u8; 16],
}
#[derive(Default,Copy,Clone)]
#[repr(C)]
pub struct NetworkRoute
{
	pub network: [u8; 16],
	pub gateway: [u8; 16],
	pub addr_ty: u8,
	pub mask: u8,
	//pub interface: u16,
}

