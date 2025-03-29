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
#[derive(Debug)]
pub struct AddrParseError(());
impl ::alloc::str::FromStr for IpAddr {
	type Err = AddrParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if let Ok(v) = s.parse() {
			Ok(IpAddr::V4(v))
		}
		else if let Ok(v) = s.parse() {
			Ok(IpAddr::V6(v))
		}
		else {
			Err(AddrParseError(()))
		}
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
impl ::alloc::str::FromStr for Ipv6Addr {
	type Err = AddrParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		const E: AddrParseError = AddrParseError(());
		if let Some((head,tail)) = s.split_once("::") {
			let (head_len, head_words) = parse_seq(head)?;
			let (tail_len, tail_words) = parse_seq(tail)?;
			if head_len == 8 || tail_len == 8 {
				return Err(E);
			}
			if head_len + tail_len > 8 {
				return Err(E);
			}
			
			let mut words = [0; 8];
			words[..head_len].copy_from_slice(&head_words[..head_len]);
			words[8 - tail_len..].copy_from_slice(&tail_words[..tail_len]);
			return Ok(Ipv6Addr { words });
		}
		else {
			let (len, words) = parse_seq(s)?;
			if len != 8 {
				return Err(E);
			}
			return Ok(Ipv6Addr { words })
		}

		fn parse_seq(s: &str) -> Result<(usize,[u16; 8]),AddrParseError> {
			let mut i = 0;
			let mut rv = [0; 8];
			let mut it = s.split(':');
			while let Some(s) = it.next() {
				if i == 8 {
					return Err(E);
				}
				rv[i] = parse_u16(16, s).map_err(|_| E)?;
				i += 1;
			}
			Ok((i, rv))
		}
	}
}

pub struct Ipv4Addr {
	bytes: [u8; 4],
}
impl Ipv4Addr {
	pub fn from_octets(a: u8, b: u8, c: u8, d: u8) -> Self {
		Ipv4Addr { bytes: [a,b,c,d] }
	}
}
impl ::core::fmt::Display for Ipv4Addr {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{},{},{},{}", self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3])
	}
}
impl ::alloc::str::FromStr for Ipv4Addr {
	type Err = AddrParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		const E: AddrParseError = AddrParseError(());
		let mut it = s.split('.');
		let rv = Ipv4Addr {
			bytes: [
				parse_u8(10, it.next().ok_or(E)?).map_err(|_| E)?,
				parse_u8(10, it.next().ok_or(E)?).map_err(|_| E)?,
				parse_u8(10, it.next().ok_or(E)?).map_err(|_| E)?,
				parse_u8(10, it.next().ok_or(E)?).map_err(|_| E)?,
			],
		};
		if let Some(_) = it.next() {
			// Extra
			return Err(E);
		}
		Ok(rv)
	}
}

fn parse_int(base: u32, s: &str) -> Result<u32,()> {
	let mut it = s.chars();
	let mut rv: u32 = 0;
	while let Some(c) = it.next() {
		rv = rv.checked_mul(base).ok_or(())?;
		let Some(v) = c.to_digit(base) else { return Err(()) };
		rv = rv.checked_add(v).ok_or(())?;
	}
	Ok(rv)
}
fn parse_u8(base: u32, s: &str) -> Result<u8,()> {
	match ::core::convert::TryInto::try_into(parse_int(base, s)?)
	{
	Ok(v) => Ok(v),
	Err(_) => Err(()),
	}
}
fn parse_u16(base: u32, s: &str) -> Result<u16,()> {
	match ::core::convert::TryInto::try_into(parse_int(base, s)?)
	{
	Ok(v) => Ok(v),
	Err(_) => Err(()),
	}
}

fn cvt_error(e: ::syscalls::values::SocketError) -> crate::io::Error {
	e.into()
}