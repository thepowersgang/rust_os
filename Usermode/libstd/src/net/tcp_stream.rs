
pub struct TcpStream(::syscalls::net::ConnectedSocket);

impl TcpStream
{
	pub fn from_syscall(h: ::syscalls::net::ConnectedSocket) -> Result<TcpStream, crate::io::Error>
	{
		Ok( TcpStream(h) )
	}
	pub fn connect(addr: impl Into<super::IpAddr>, port: u16) -> Result<TcpStream, crate::io::Error>
	{
		let mut addr_data = [0; 16];
		let addr_ty = match addr.into() {
			crate::net::IpAddr::V4(ipv4_addr) => {
				addr_data[..4].copy_from_slice(&ipv4_addr.bytes);
				::syscalls::values::SocketAddressType::Ipv4 as _
			},
			crate::net::IpAddr::V6(ipv6_addr) => {
				for (d,w) in Iterator::zip( addr_data.chunks_mut(2), ipv6_addr.words.iter() ) {
					d.copy_from_slice(&w.to_be_bytes());
				}
				::syscalls::values::SocketAddressType::Ipv6 as _
			}
			};
		let sa = ::syscalls::values::SocketAddress {
			port_ty: ::syscalls::values::SocketPortType::Tcp as _,
			port,
			addr_ty,
			addr: addr_data,
		};
		// Ask the handle server for a connection?
		// Or just open the socket from kernel
		let h = ::syscalls::net::ConnectedSocket::connect(sa).map_err(super::cvt_error)?;
		Ok(TcpStream(h))
	}
}

impl crate::io::Read for TcpStream
{
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		(&*self).read(buf)
	}
}
impl crate::io::Write for TcpStream
{
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		(&*self).write(buf)
	}

	fn flush(&mut self) -> io::Result<()> {
		(&*self).flush()
	}
}
impl<'a> crate::io::Read for &'a TcpStream
{
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		self.0.recv(buf).map_err(super::cvt_error)
	}
}
impl<'a> crate::io::Write for &'a TcpStream
{
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		self.0.send(buf).map_err(super::cvt_error)
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}


