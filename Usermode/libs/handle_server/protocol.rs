// Tifflin OS - handle_server Library
// - By John Hodge (thePowersGang)
//
// libhandle_server/protocol.rs
//! "Wire" protocol definitions

macro_rules! def_message_transmute {
	($ty:ty) => {
		impl From<$ty> for ::syscalls::ipc::RpcMessage {
			fn from(v: $ty) -> Self {
				// SAFE: Source should be POD
				unsafe { ::core::mem::transmute(v) }
			}
		}
	};
}
macro_rules! def_proto_type {
	($enm:ident :: $name:ident => $typename:ident
		struct {
			$($n:ident: $ty:ty,)*
		}
		new($($arg:ident: $arg_ty:ty),*) {
			$($new_n:ident: $new_e:expr,)*
		}
		try_from($v:ident) {
			$($from_n:ident: $from_e:expr,)*
		}
		)
	=>
	{
		#[repr(packed)]
		pub struct $typename
		{
			_rsp_id: $enm,
			$($n: $ty,)*
		}
		impl $typename
		{
			pub fn new($($arg: $arg_ty),*) -> $typename {
				$typename {
					_rsp_id: $enm::$name,
					$($new_n: $new_e,)*
					}
			}
			pub fn try_from($v: ::syscalls::ipc::RpcMessage) -> Option<Self> {
				Some($typename {
					_rsp_id: $enm::$name,
					$($from_n: $from_e,)*
					})
			}
		}
	}
}

pub enum UnmarshalError
{
	UnknownRequest,
	BadValue,
}

pub enum Request
{
	CreateChild(ReqCreateChild),
	OpenExecutable(ReqOpenExecutable),
	PickFile(ReqPickFile),
}
impl Request
{
	pub fn try_from(v: ::syscalls::ipc::RpcMessage) -> Result<Self, UnmarshalError> {
		match RequestId::try_from(v[0])
		{
		Some(RequestId::CreateChild) => ReqCreateChild::try_from(v).ok_or(UnmarshalError::BadValue).map(Request::CreateChild),
		Some(RequestId::OpenExecutable) => match ReqOpenExecutable::try_from(v)
			{
			Some(rv) => Ok(Request::OpenExecutable(rv)),
			None => Err(UnmarshalError::BadValue),
			},
		Some(RequestId::PickFile) => match ReqPickFile::try_from(v)
			{
			Some(rv) => Ok(Request::PickFile(rv)),
			None => Err(UnmarshalError::BadValue),
			},
		None => Err(UnmarshalError::UnknownRequest),
		}
	}
}

#[repr(u8)]
enum RequestId
{
	CreateChild,
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

def_proto_type! {
	RequestId::CreateChild => ReqCreateChild
	struct {
		name_buf: [u8; 31],
	}
	new(name: &str) {
		name_buf: zero_pad_bytes_into(name.as_bytes()),
	}
	try_from(v) {
		name_buf: zero_pad_bytes_into(&v[1..]),
	}
}
def_message_transmute! { ReqCreateChild }
impl ReqCreateChild
{
	pub fn name(&self) -> &[u8] {
		get_zero_terminated_slice(&self.name_buf)
	}
}

def_proto_type! {
	RequestId::OpenExecutable => ReqOpenExecutable
	struct {
		name_buf: [u8; 31],
	}
	new(name: &str) {
		name_buf: zero_pad_bytes_into(name.as_bytes()),
	}
	try_from(v) {
		name_buf: zero_pad_bytes_into(&v[1..]),
	}
}
def_message_transmute! { ReqOpenExecutable }
impl ReqOpenExecutable
{
	pub fn name(&self) -> &[u8] {
		get_zero_terminated_slice(&self.name_buf)
	}
}

def_proto_type! {
	RequestId::PickFile => ReqPickFile
	struct {
		mode: PickFileMode,
		description: [u8; 30],
	}
	new(mode: PickFileMode, desc: &str) {
		mode: mode,
		description: zero_pad_bytes_into(desc.as_bytes()),
	}
	try_from(v) {
		mode: match PickFileMode::try_from(v[1])
			{
			Some(v) => v,
			None => return None,
			},
		description: zero_pad_bytes_into(&v[2..]),
	}
}
impl ReqPickFile
{
	pub fn mode(&self) -> PickFileMode {
		self.mode
	}
	pub fn description_raw(&self) -> &[u8] {
		get_zero_terminated_slice(&self.description)
	}
}
#[repr(u8)]
#[derive(Copy,Clone,Debug)]
pub enum PickFileMode
{
	ReadOnly,
	ReadWrite,
	Create,
	OptionalWrite,
}
impl PickFileMode
{
	pub fn try_from(v: u8) -> Option<Self> {
		if v <= PickFileMode::OptionalWrite as u8 {
			// SAFE: Range checked
			Some(unsafe { ::core::mem::transmute(v) })
		}
		else {
			None
		}
	}
}



pub enum Response
{
	Error(RspError),
	OpenedFile(RspOpenedFile),
	NewChannel(RspNewChannel),
}
impl Response
{
	pub fn try_from(v: ::syscalls::ipc::RpcMessage) -> Result<Self, UnmarshalError> {
		macro_rules! handle_responses {
			($v:ident : $($n:ident => $t:ident,)+) => {
				match ResponseId::try_from(v[0])
				{
				$( Some(ResponseId::$n) => $t::try_from(v).ok_or(UnmarshalError::BadValue).map(|x| Response::$n(x)), )+
				None => Err(UnmarshalError::UnknownRequest),
				}
			}
		}
		handle_responses!(v :
			Error => RspError,
			OpenedFile => RspOpenedFile,
			NewChannel => RspNewChannel,
			)
	}
}

#[repr(u8)]
enum ResponseId
{
	Error,
	OpenedFile,
	NewChannel,
}
impl ResponseId
{
	pub fn try_from(v: u8) -> Option<Self> {
		if v <= ResponseId::OpenedFile as u8 {
			// SAFE: Range checked
			Some(unsafe { ::core::mem::transmute(v) })
		}
		else {
			None
		}
	}
}

def_proto_type! {
	ResponseId::Error => RspError
	struct {
		error_id: u8,
		message: [u8; 30],
	}
	new(code: u8, msg: &str) {
		error_id: code,
		message: zero_pad_bytes_into(msg.as_bytes()),
	}
	try_from(v) {
		error_id: v[1],
		message: zero_pad_bytes_into(&v[2..]),
	}
}
def_message_transmute! { RspError }
impl RspError
{
	pub fn error_id(&self) -> u8 {
		self.error_id
	}
	pub fn message(&self) -> &str {
		::core::str::from_utf8( get_zero_terminated_slice(&self.message) ).expect("Invalid UTF-8 from handle server")
	}
}
impl ::core::fmt::Debug for RspError
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "RspError({} {})", self.error_id, self.message())
	}
}

def_proto_type! {
	ResponseId::OpenedFile => RspOpenedFile
	struct {
		filename: [u8; 31],
	}
	new(path: &[u8]) {
		filename: zero_pad_bytes_into(path),
	}
	try_from(v) {
		filename: zero_pad_bytes_into(&v[1..]),
	}
}
def_message_transmute! { RspOpenedFile }
impl RspOpenedFile
{
	pub fn filename(&self) -> &[u8] {
		get_zero_terminated_slice(&self.filename)
	}
}

def_proto_type! {
	ResponseId::NewChannel => RspNewChannel
	struct {
		_unused: [u8; 31],
	}
	new() {
		_unused: [0; 31],
	}
	try_from(_v) {
		_unused: [0; 31],
	}
}
def_message_transmute! { RspNewChannel }

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

