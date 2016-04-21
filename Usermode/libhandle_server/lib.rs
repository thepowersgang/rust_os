// Tifflin OS - handle_server Library
// - By John Hodge (thePowersGang)
//
// libhandle_server/lib.rs
//! Interface library for the "Handle server"
#![no_std]

extern crate syscalls;

#[doc(hidden)]
pub mod protocol;

pub struct Connection
{
	channel: ::syscalls::ipc::RpcChannel,
}

pub enum OpenError
{
	Cancelled,
	NotFound,
	PermissionDenied,
}

impl Connection
{
	/// Create a new connection by receiving the handle from the parent process
	pub fn rx_new() -> Connection {
		Self::new( ::syscalls::threads::S_THIS_PROCESS.receive_object().expect("Failed to receive handle_server connection") )
	}
	/// Create a new connection using the provided RPC Channel handle
	pub fn new(channel: ::syscalls::ipc::RpcChannel) -> Connection {
		Connection {
			channel: channel,
		}
	}
}


/// Blocking requests
impl Connection
{
	/// Open a named executable
	pub fn open_executable(&self, name: &str) -> Result< ::syscalls::vfs::File, OpenError > {
		self.channel.send( protocol::RequestExecutable::new(name).into() );
		::syscalls::threads::wait(&mut [ self.channel.wait_rx() ], !0);
		let (_rsp, obj) = self.channel.try_receive().unwrap();
		if let Some(v) = obj {
			// TODO TODO TODO Use a proper type here that can be checked-casted
			Ok( ::syscalls::Object::from_handle(v) )
		}
		else {
			Err( OpenError::NotFound )
		}
	}

	/// Ask the user to select a file for reading
	pub fn select_file_ro(&self, reason: &str) -> Result< ::syscalls::vfs::File, OpenError > {
		unimplemented!()
	}
	/// Ask the user to a select a file to edit (read+write)
	pub fn select_file_rw(&self, reason: &str) -> Result< ::syscalls::vfs::File, OpenError > {
		unimplemented!()
	}
	/// As the user to select a file to optionally edit (can return a ReadOnly handle)
	pub fn select_file_maybe_write(&self, reason: &str) -> Result< ::syscalls::vfs::File, OpenError > {
		unimplemented!()
	}
	/// Ask the user to select an output filename (for creation/over-write)
	pub fn select_file_new(&self, reason: &str) -> Result< ::syscalls::vfs::File, OpenError > {
		unimplemented!()
	}
}

