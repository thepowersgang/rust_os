use std::convert::TryInto;

#[derive(Debug)]
pub enum Opt<'a> {
	Malformed(u8, &'a [u8]),
	Unknown(u8, &'a [u8]),
	//Pad,	// #0

	/// Subnet mask, encoded as an address
	SubnetMask([u8; 4]),
	/// Timezone seconds east of UTC
	TimeOffset(i32),
	/// A list of routers
	Routers(&'a [[u8;4]]),
	/// A list of ?NTP time servers
	TimeServers(&'a [[u8;4]]),
	/// A list of IEN-116 name servers
	NameServersIen116(&'a [[u8;4]]),
	/// #6 A list of DNS name servers
	NameServersDns(&'a [[u8;4]]),

	/// #12 Client hostname
	HostName(&'a [u8]),
	/// #15 Domain Name
	DomainName(&'a [u8]),

	/// #42 Vendor-specific
	VendorSpecific(&'a [u8]),

	/// #50 Allows client to request a specific IP address
	RequestedIpAddress([u8; 4]),

	/// #52 Specifies that `file` or `sname` (or both) also contain options
	/// 
	/// 1 = only `file`
	/// 2 = only `sname`
	/// 3 = both
	OptionOverload(u8),

	/// #53 
	DhcpMessageType(u8),

	/// #54 IPv4 address of the server that sent this offer/ack
	ServerIdentifier([u8; 4]),
	/// #55 - Parameters to request, as option indexes
	ParameterRequestList(&'a [u8]),
	/// #56 - Human-readable (ASCII) text error message for DHCPNAK
	Message(&'a [u8]),

	/// #61 - An opaque blob client identifier
	ClientIdentifier(&'a [u8]),
}
macro_rules! enc_dec_option {
	($in_data:ident ;
	$(
		$idx:literal $name:ident( $valname:ident ) : $dec:expr => $enc:expr ;
	)*
	) => {
		#[allow(dead_code)]
		#[allow(non_upper_case_globals)]
		pub mod codes {
			$(pub const $name: u8 = $idx;)*
		}
		impl<'a> Opt<'a> {
			pub fn decode(code: u8, $in_data: &'a [u8]) -> Self {
				match code {
				0 => unreachable!(),
				$(
				$idx => if let Some(v) = $dec { Opt::$name(v) } else { Opt::Malformed(code, $in_data) },
				)*
				_ => Opt::Unknown(code, $in_data)
				}
			}
			pub fn encode(&self, mut push: impl FnMut(u8, &[u8])) {
				fn flatten(data: &[[u8; 4]]) -> &[u8] {
					// SAFE: Same alignment, correct length
					unsafe { ::core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len()*4) }
				}
				match self {
				Opt::Malformed(_op, _data) => {
					// Ignore malformed data
				}
				Opt::Unknown(op, data) => push(*op, data),
				$(
				Opt::$name($valname) => push($idx, $enc),
				)*
				}
			}
		}
	}
}
fn get_u8_4(data: &[u8]) -> Option<&[u8; 4]> {
	data.try_into().ok()
}
fn get_u8_4_seq(data: &[u8]) -> Option<&[[u8;4]]> {
	if data.len() % 4 == 0 {
		// SAFE: Same alignment, and length is aligned to 4
		Some(unsafe { ::core::slice::from_raw_parts(data.as_ptr() as *const [u8; 4], data.len() / 4) })
	}
	else {
		None
	}
}
enc_dec_option!{d;
	// 0 Pad (not encoded)
	1 SubnetMask(data) : get_u8_4(d).copied() => data;
	2 TimeOffset(ofs) : get_u8_4(d).map(|v| i32::from_be_bytes(*v)) => &ofs.to_le_bytes();
	3 Routers(addrs)           : get_u8_4_seq(d) => flatten(addrs);
	4 TimeServers(addrs)       : get_u8_4_seq(d) => flatten(addrs);
	5 NameServersIen116(addrs) : get_u8_4_seq(d) => flatten(addrs);
	6 NameServersDns(addrs)    : get_u8_4_seq(d) => flatten(addrs);
	12 HostName(name)   : Some(d) => name;
	15 DomainName(name) : Some(d) => name;
	42 VendorSpecific(data) : Some(d) => data;
	50 RequestedIpAddress(addr) : get_u8_4(d).copied() => addr;
	52 OptionOverload(v)  : match d { &[v] => Some(v), _ => None } => &[*v];
	53 DhcpMessageType(v) : match d { &[v] => Some(v), _ => None } => &[*v];
	54 ServerIdentifier(data) : get_u8_4(d).copied() => data;
	55 ParameterRequestList(params) : Some(d) => params;
	56 Message(msg) : Some(d) => msg;
	61 ClientIdentifier(blob) : Some(d) => blob;
	// 255 End (not encoded)
}

#[derive(Clone)]
pub struct OptionsIter<'a>(pub &'a [u8]);
impl<'a> ::std::fmt::Debug for OptionsIter<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("OptionsIter(")?;
		let mut len = None;
		for b in self.0 {
			len = match len {
				None => {
					f.write_str(" ")?;
					// If the option ID isn't 0 (pad), then update the length to be non-negative so the next iteration
					// gets the length
					if *b != 0 {
						Some(0)
					}
					else {
						None
					}
					}
				Some(0) => {
					if *b == 0 {
						None
					}
					else {
						Some(*b)
					}
				},
				Some(l) => {
					if l == 1 {
						None
					}
					else {
						Some(l - 1)
					}
				}
				};
			write!(f, "{:02x}", b)?;
		}
		f.write_str(" )")
	}
}
impl<'a> Iterator for OptionsIter<'a> {
	type Item = Opt<'a>;
	fn next(&mut self) -> Option<Self::Item> {
		match self.0 {
		[] => None,
		// Padding: Handled specially
		[0, tail @ ..] => {
			self.0 = tail;
			//Some(Opt::Pad)
			self.next()
		},
		// End
		[255, ..] => None,
		[_] => {
			self.0 = &[];
			None
		},
		&[code, len, ref tail @ ..] => {
			let Some( (data,tail) ) = tail.split_at_checked(len as usize) else {
				self.0 = &[];
				return None;
			};
			self.0 = tail;
			Some(Opt::decode(code, data))
		}
		}
	}
}