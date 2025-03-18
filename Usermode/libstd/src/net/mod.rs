//
//
//

pub use self::tcp_stream::TcpStream;

pub struct Error;

pub struct TcpListener;
pub struct UdpSocket;

mod tcp_stream;

pub enum IpAddr
{
	V4(Ipv4Addr),
	V6(Ipv6Addr),
}
impl From<Ipv6Addr> for IpAddr
{
	fn from(v: Ipv6Addr) -> IpAddr {
		IpAddr::V6(v)
	}
}
impl From<Ipv4Addr> for IpAddr
{
	fn from(v: Ipv4Addr) -> IpAddr {
		IpAddr::V4(v)
	}
}

pub struct Ipv6Addr
{
	words: [u16; 8],
}
impl Ipv6Addr
{
	fn get_zero_block(&self) -> ::core::ops::Range<usize> {
		// TODO: Use a generator and max?
		let mut longest = (0, 0);	// Offset, len
		let mut cur = (0, 0);	// Offset, len
		for (i,&v) in self.words.iter().enumerate()
		{
			if v == 0 {
				if cur.1 == 0 {
					cur.0 = i;
				}
				cur.1 += 1;
			}
			else {
				if cur.1 > longest.1 {
					longest = cur;
				}
				cur.1 = 0;
			}
		}
		if cur.1 > longest.1 {
			longest = cur;
		}
		longest.0 .. (longest.0 + longest.1)
	}
}
impl ::core::fmt::Display for Ipv6Addr
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		let zero_rgn = self.get_zero_block();
		for (i,v) in self.words.iter().enumerate() {
			if zero_rgn.contains(&i) {
				if i == zero_rgn.start {
					f.write_str(":")?;
				}
			}
			else {
				if i != 0 {
					f.write_str(":")?;
				}
				v.fmt(f)?;
			}
		}
		Ok( () )
	}
}

pub struct Ipv4Addr
{
	bytes: [u8; 4],
}
impl ::core::fmt::Display for Ipv4Addr
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{},{},{},{}", self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3])
	}
}


fn cvt_error(e: ::syscalls::values::SocketError) -> crate::io::Error {
	e.into()
}