// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
// net.rs
/// User->Kernel network connection interface

pub use ::values::SocketError as Error;
pub use ::values::SocketShutdownSide as ShutdownSide;
pub use ::values::SocketAddress as SocketAddress;
pub use ::values::MaskedSocketAddress;

/// Network connection server (allows waiting for an incoming connection)
pub struct Server(::ObjectHandle);
/// Handle to an active socket connection (e.g. TCP)
pub struct ConnectedSocket(::ObjectHandle);
/// Handle to an acive free connection (e.g. UDP)
pub struct FreeSocket(::ObjectHandle);

fn to_result(val: usize) -> Result<u32, Error> {
	::to_result(val).map_err(|e| Error::try_from(e as u8).unwrap())
}

// --------------------------------------------------------------------
impl ::Object for Server
{
	const CLASS: u16 = ::values::CLASS_SERVER;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		Server(handle)
	}
	fn into_handle(self) -> ::ObjectHandle {
		self.0
	}
	fn handle(&self) -> &::ObjectHandle {
		&self.0
	}

	type Waits = ();
}
impl Server
{
	pub fn open(addr: impl Into<SocketAddress>) -> Result<Server, Error> {
		let addr = addr.into();
		// SAFE: Syscall
		::ObjectHandle::new(unsafe { syscall!(NET_LISTEN, &addr as *const _ as usize) as usize })
			.map_err(|e| Error::try_from(e as u8).unwrap() )
			.map(|v| Server(v))
	}

	pub fn accept(&self) -> Result<(ConnectedSocket, SocketAddress), Error> {
		let mut sa = SocketAddress::default();
		// SAFE: Syscall
		::ObjectHandle::new(unsafe { self.0.call_1(::values::NET_SERVER_ACCEPT, &mut sa as *mut _ as usize) as usize })
			.map_err(|e| Error::try_from(e as u8).unwrap() )
			.map( |v| (ConnectedSocket(v), sa,) )
	}
}
// --------------------------------------------------------------------
impl ::Object for ConnectedSocket
{
	const CLASS: u16 = ::values::CLASS_SOCKET;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		ConnectedSocket(handle)
	}
	fn into_handle(self) -> ::ObjectHandle {
		self.0
	}
	fn handle(&self) -> &::ObjectHandle {
		&self.0
	}

	type Waits = ();
}
impl ConnectedSocket
{
	pub fn connect(addr: impl Into<SocketAddress>) -> Result<ConnectedSocket, Error> {
		let addr = addr.into();
		// SAFE: Syscall
		::ObjectHandle::new(unsafe{ syscall!(NET_CONNECT, &addr as *const _ as usize) as usize })
			.map_err(|e| Error::try_from(e as u8).unwrap())
			.map(|v| ConnectedSocket(v))
	}
	pub fn shutdown(&self, what: ShutdownSide) -> Result<(), Error> {
		// SAFE: Syscall
		to_result(unsafe { self.0.call_1(::values::NET_CONNSOCK_SHUTDOWN, what as usize) as usize })
			.map(|_| ())
	}

	//pub fn set_read_timeout(&self, timeout: Option<Duration>) {
	//}
	//pub fn get_read_timeout(&self, timeout: Option<Duration>) {
	//}
	//pub fn set_write_timeout(&self, timeout: Option<Duration>) {
	//}
	//pub fn get_write_timeout(&self, timeout: Option<Duration>) {
	//}
}
impl ConnectedSocket
{
	pub fn send(&mut self, data: &[u8]) -> Result<usize, Error> {
		// SAFE: Syscall
		to_result(unsafe { self.0.call_2(::values::NET_CONNSOCK_SEND, data.as_ptr() as usize, data.len()) as usize })
			.map(|v| v as usize)
	}
	pub fn recv(&mut self, data: &mut [u8]) -> Result<usize, Error> {
		// SAFE: Syscall
		to_result(unsafe { self.0.call_2(::values::NET_CONNSOCK_RECV, data.as_ptr() as usize, data.len()) as usize })
			.map(|v| v as usize)
	}

	// TODO: Async IO using registered buffers (which minimises the problems with borrowing)
}
// --------------------------------------------------------------------
impl ::Object for FreeSocket
{
	const CLASS: u16 = ::values::CLASS_FREESOCKET;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: ::ObjectHandle) -> Self {
		FreeSocket(handle)
	}
	fn into_handle(self) -> ::ObjectHandle {
		self.0
	}
	fn handle(&self) -> &::ObjectHandle {
		&self.0
	}

	type Waits = ();
}
impl FreeSocket
{
	/// Create a free socket using the specified local and remote addresses.
	// TODO: Rx masks (as opposed to either a specific address or wildcard)
	// - Could also register mask sets?
	pub fn create(local: SocketAddress, remote: MaskedSocketAddress) -> Result<FreeSocket, Error> {
		// SAFE: Syscall
		::ObjectHandle::new( unsafe { syscall!(NET_BIND, &local as *const _ as usize, &remote as *const _ as usize) as usize } )
			.map_err(|e| Error::try_from(e as u8).unwrap())
			.map(|v| FreeSocket(v))
	}

	pub fn send_to(&mut self, data: &[u8], remote: SocketAddress) -> Result<usize, Error> {
		// SAFE: Syscall
		to_result( unsafe { self.0.call_3(::values::NET_FREESOCK_SEND, data.as_ptr() as usize, data.len(), &remote as *const _ as usize) as usize } )
			.map(|v| v as usize)
	}
	pub fn recv_from(&mut self, data: &mut [u8]) -> Result<(usize, SocketAddress), Error> {
		let mut sa = SocketAddress::default();
		// SAFE: Syscall
		to_result( unsafe { self.0.call_3(::values::NET_FREESOCK_RECV, data.as_ptr() as usize, data.len(), &mut sa as *mut _ as usize) as usize } )
			.map(|v| (v as usize, sa))
	}
}

