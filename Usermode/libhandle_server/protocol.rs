// Tifflin OS - handle_server Library
// - By John Hodge (thePowersGang)
//
// libhandle_server/protocol.rs
//! "Wrire" protocol definitions

macro_rules! def_message_transmute {
	($ty:ty) => {
		// TODO: This isn't always safe (see internal enums)
		impl From<::syscalls::ipc::RpcMessage> for $ty {
			fn from(v: ::syscalls::ipc::RpcMessage) -> Self {
				// SAFE: (TODO) By calling this macro, the invoker says this is safe
				unsafe { ::core::mem::transmute(v) }
			}
		}
		impl From<$ty> for ::syscalls::ipc::RpcMessage {
			fn from(v: $ty) -> Self {
				// SAFE: Source should be POD
				unsafe { ::core::mem::transmute(v) }
			}
		}
	};
}

#[repr(u8)]
pub enum RequestId
{
	OpenExecutable,
	PickFile,
}
impl RequestId
{
	pub fn try_from(v: u8) -> Option<RequestId> {
		if v <= RequestId::PickFile as u8 {
			// SAFE: Range checked
			Some(unsafe { ::core::mem::transmute(v) })
		}
		else {
			None
		}
	}
}

#[repr(packed)]
pub struct RequestExecutable
{
 	#[allow(dead_code)]
	request_id: RequestId,
	name_buf: [u8; 31],
}
def_message_transmute! { RequestExecutable }
impl RequestExecutable
{
	pub fn new(name: &str) -> RequestExecutable {
		RequestExecutable {
			request_id: RequestId::OpenExecutable,
			name_buf: zero_pad_bytes_into(name.as_bytes()),
			}
	}

	pub fn name(&self) -> &[u8] {
		get_zero_terminated_slice(&self.name_buf)
	}
}

#[repr(u8)]
pub enum ResponseId
{
	Error,
	File,
}

#[repr(packed)]
pub struct RspError
{
 	#[allow(dead_code)]
	rsp_id: ResponseId,
	error_id: u8,
	message: [u8; 30],
}
def_message_transmute! { RspError }
impl RspError
{
	pub fn new(code: u8, msg: &str) -> RspError {
		RspError {
			rsp_id: ResponseId::Error,
			error_id: code,
			message: zero_pad_bytes_into(msg.as_bytes()),
			}
	}

	pub fn error_id(&self) -> u8 {
		self.error_id
	}
	pub fn message(&self) -> &str {
		::core::str::from_utf8( get_zero_terminated_slice(&self.message) ).expect("Invalid UTF-8 from handle server")
	}
}

pub struct RspFile
{
 	#[allow(dead_code)]
	rsp_id: ResponseId,
	filename: [u8; 31],
}
impl RspFile
{
	pub fn new(path: &[u8]) -> RspFile {
		RspFile {
			rsp_id: ResponseId::File,
			filename: zero_pad_bytes_into(path),
			}
	}

	pub fn filename(&self) -> &[u8] {
		get_zero_terminated_slice(&self.filename)
	}
}
def_message_transmute! { RspFile }


fn zero_pad_bytes_into<T: Default+AsMut<[u8]>>(src: &[u8]) -> T {
	let mut rv = T::default();
	for (&b, d) in Iterator::zip( src.iter(), rv.as_mut().iter_mut() ) {
		*d = b;
	}
	rv
}

fn get_zero_terminated_slice(i: &[u8]) -> &[u8] {
	let l = i.iter().position(|&x|x==0).unwrap_or( i.len() );
	&i[..l]
}

