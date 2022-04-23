// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/ipv4.rs
//! IPv4 (Layer 3)
use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::MacAddr;
use crate::nic::PacketReader;

// List of protocol numbers and handlers
static PROTOCOLS: RwLock<Vec<(u8, ProtoHandler)>> = RwLock::new(Vec::new_const());
static INTERFACES: RwLock<Vec<Interface>> = RwLock::new(Vec::new_const());

// NOTE: uses mac address to identify interface
pub fn add_interface(local_mac: [u8; 6], addr: Address, mask_bits: u8)
{
	let mut lh = INTERFACES.write();
	for interface in lh.iter()
	{
		if interface.address == addr
		{
			// Whups?
			return ;
		}
	}

	lh.push(Interface {
		local_mac: local_mac,
		address: addr,
		mask: mask_bits,
		});
}

pub fn register_handler(proto: u8, handler: fn(&Interface, Address, PacketReader)) -> Result<(), ()>
{
	let mut lh = PROTOCOLS.write();
	for &(p, _) in lh.iter()
	{
		if p == proto {
			return Err( () );
		}
	}
	lh.push( (proto, ProtoHandler::DirectKernel(handler),) );
	Ok( () )
}
pub fn handle_rx_ethernet(_physical_interface: &dyn crate::nic::Interface, source_mac: [u8; 6], mut reader: PacketReader) -> Result<(), ()>
{
	let pre_header_reader = reader.clone();
	let hdr = match Ipv4Header::read(&mut reader)
		{
		Ok(v) => v,
		Err(_) => {
			log_warning!("Undersized packet: Ran out of data reading header");
			return Err( () );
			},
		};
	
	if hdr.ver_and_len >> 4 != 4 {
		// Malformed packet, bad IP version
		log_warning!("Malformed packet: version isn't 4 - ver_and_len={:02x}", hdr.ver_and_len);
		return Err( () );
	}
	let hdr_len = hdr.get_header_length();
	if hdr_len > pre_header_reader.remain()
	{
		// Malformed packet, header's reported size is larger than the buffer
		log_warning!("Malformed packet: header over-sized ({} > {})", hdr_len, pre_header_reader.remain());
		return Err( () );
	}
	
	if reader.remain() > pre_header_reader.remain() - hdr_len
	{
		// Consume bytes out of the buffer until the end of the header is reached
		// - I.e. the remaning byte count equals the number of bytes after the header 
		let n_bytes_after_header = pre_header_reader.remain() - hdr_len;
		while reader.remain() > n_bytes_after_header
		{
			// TODO: options!
			match reader.read_u8()?
			{
			_ => {},
			}
		}
	}
	
	// Validate checksum: Sum all of the bytes
	{
		let mut reader2 = pre_header_reader.clone();
		let sum = calculate_checksum( (0 .. hdr_len/2).map(|_| reader2.read_u16n().unwrap()) );
		if sum != 0 {
			log_warning!("IP Checksum failure - sum is {:#x}, not zero", sum);
		}
	}
	
	// Check for IP-level fragmentation
	if hdr.get_has_more_fragments() || hdr.get_fragment_ofs() != 0 {
		// TODO: Handle fragmented packets
		log_error!("TODO: Handle fragmented packets");
		return Ok( () );
	}
	
	// Sanity check that we have enough bytes for the body.
	if reader.remain() < hdr.total_length as usize - hdr_len {
		log_warning!("Undersized packet: {} bytes after header, body length is {}", reader.remain(), hdr.total_length as usize - hdr_len);
		return Err( () );
	}

	
	// Check destination IP against known interfaces.
	// - Could also be doing routing.
	for interface in INTERFACES.read().iter()
	{
		// TODO: Interfaces should be locked to the physical interface too
		if interface.address == hdr.destination
		{
			// TODO: Should there be per-interface handlers?

			// TODO: Check if the source address is from the same subnet, and only cache in ARP if it is
			crate::arp::snoop_v4(source_mac, hdr.source);

			// Figure out which sub-protocol to send this packet to
			// - Should there be alternate handlers for 
			for &(id,ref handler) in PROTOCOLS.read().iter()
			{
				if id == hdr.protocol
				{
					handler.dispatch(interface, hdr.source, hdr.destination, reader);
					return Ok( () );
				}
			}
			log_debug!("Unknown protocol {}", hdr.protocol);
			// No handler, but the interface is known
			return Ok( () );
		}
	}
	//else
	{
		// Routing.
		// For now, just drop it
		log_debug!("TODO: Packet didn't match any interfaces (A={:?}), try routing?", hdr.destination);
	}
	
	Ok( () )
}

// Calculate a checksum of a sequence of NATIVE ENDIAN (not network) 16-bit words
pub fn calculate_checksum(words: impl Iterator<Item=u16>) -> u16
{
	let mut sum = 0;
	for v in words
	{
		sum += v as usize;
	}
	while sum > 0xFFFF
	{
		sum = (sum & 0xFFFF) + (sum >> 16);
	}
	!sum as u16
}

// 
pub fn route_lookup(source: Address, dest: Address) -> Option<(Address, MacAddr, Address)>
{
	for interface in INTERFACES.read().iter()
	{
		// On-link?
		if (source.is_zero() || interface.address == source) && interface.address.mask(interface.mask) == dest.mask(interface.mask)
		{
			return Some( (interface.address, interface.local_mac, dest) );
		}
	}
	None
}

pub async fn send_packet(source: Address, dest: Address, proto: u8, pkt: crate::nic::SparsePacket<'_>)
{
	log_trace!("send_packet({:?} -> {:?} 0x{:02x})", source, dest, proto);
	// 1. Look up routing table for destination IP and interface
	let (_, interface_mac, next_hop) = match route_lookup(source, dest)
		{
		Some(v) => v,
		None => {
			log_notice!("Unable to send to {:?}: No route", dest);
			return	// TODO: Error - No route to host
			},
		};
	// 2. ARP (what if ARP has to wait?)
	let dest_mac = match crate::arp::lookup_v4(interface_mac, next_hop).await
		{
		Some(v) => v,
		None => {
			log_notice!("Unable to send to {:?}: No ARP", dest);
			return
			},	// TODO: Error - No route to host
		};
	// 3. Send
	let mut hdr = Ipv4Header {
		ver_and_len: 0x40 | 20/4,
		diff_services: 0,
		total_length: (20 + pkt.total_len()) as u16,
		identification: 0,
		flags: 0,
		frag_ofs_high: 0,
		ttl: 255,
		protocol: proto,
		hdr_checksum: 0,
		source: source,
		destination: dest,
		};
	hdr.set_checksum();
	let hdr_bytes = hdr.encode();
	crate::nic::send_from(interface_mac, dest_mac, 0x0800, crate::nic::SparsePacket::new_chained(&hdr_bytes, &pkt));
}

#[allow(dead_code)]
struct Ipv4Header
{
	ver_and_len: u8,
	diff_services: u8,
	total_length: u16,
	identification: u16,
	flags: u8,
	frag_ofs_high: u8,
	ttl: u8,
	protocol: u8,
	hdr_checksum: u16,
	source: Address,
	destination: Address,
}
impl Ipv4Header
{
	fn encode(&self) -> [u8; 20] {
		[
			self.ver_and_len,
			self.diff_services,
			(self.total_length >> 8) as u8, self.total_length as u8,
			(self.identification >> 8) as u8, self.identification as u8,
			self.flags,
			self.frag_ofs_high,
			self.ttl,
			self.protocol,
			(self.hdr_checksum >> 8) as u8, self.hdr_checksum as u8,
			self.source.0[0], self.source.0[1], self.source.0[2], self.source.0[3],
			self.destination.0[0], self.destination.0[1], self.destination.0[2], self.destination.0[3],
			]
	}
	fn set_checksum(&mut self)
	{
		self.hdr_checksum = 0;
		self.hdr_checksum = calculate_checksum(self.encode().chunks(2).map(|v| (v[0] as u16) << 8 | v[1] as u16));
	}
	fn read(reader: &mut PacketReader) -> Result<Self, ()>
	{
		Ok(Ipv4Header {
			ver_and_len: reader.read_u8()?,
			diff_services: reader.read_u8()?,
			total_length: reader.read_u16n()?,
			identification: reader.read_u16n()?,
			flags: reader.read_u8()?,
			frag_ofs_high: reader.read_u8()?,	// low bits in the `flags` field
			ttl: reader.read_u8()?,
			protocol: reader.read_u8()?,
			hdr_checksum: reader.read_u16n()?,
			source: Address(reader.read_bytes([0; 4])?),
			destination: Address(reader.read_bytes([0; 4])?),
			})
	}

	fn get_header_length(&self) -> usize {
		(self.ver_and_len & 0xF) as usize * 4
	}
	fn get_has_more_fragments(&self) -> bool {
		self.flags & 1 << 5 != 0
	}
	//fn set_has_more_fragments(&mut self) {
	//	self.flags |= 1 << 5;
	//}

	fn get_fragment_ofs(&self) -> usize {
		((self.frag_ofs_high as usize) << 5) | (self.flags & 0x1F) as usize
	}
}

enum ProtoHandler
{
	/// Direct in-kernel handling (e.g. TCP)
	DirectKernel(fn(&Interface, Address, PacketReader)),
	/// Indirect user handling (pushes onto a buffer for the user to read from)
	// Ooh, another use for stack_dst, a DST queue!
	#[allow(dead_code)]
	User(Address, ()),
}
impl ProtoHandler
{
	fn dispatch(&self, i: &Interface, src: Address, _dest: Address, r: PacketReader)
	{
		match *self
		{
		ProtoHandler::DirectKernel(fcn) => fcn(i, src, r),
		ProtoHandler::User(..) => todo!("User-bound raw IP connections"),
		}
	}
}

#[derive(Copy,Clone,Default,PartialEq,PartialOrd,Eq,Ord)]
pub struct Address(pub [u8; 4]);
impl ::core::fmt::Display for Address
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
	}
}
impl ::core::fmt::Debug for Address {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		::core::fmt::Display::fmt(self, f)
	}
}
impl Address
{
	pub fn zero() -> Self {
		Address([0,0,0,0])
	}
	pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
		Address([a,b,c,d])
	}
	/// Big endian u32 (so 127.0.0.1 => 0x7F000001)
	pub fn as_u32(&self) -> u32 {
		(self.0[0] as u32) << 24
		| (self.0[1] as u32) << 16
		| (self.0[2] as u32) << 8
		| (self.0[3] as u32) << 0
	}
	pub fn mask(&self, bits: u8) -> Address {
		let mask = (1 << (bits % 8)) - 1;
		if bits < 8 {
			Address([ self.0[0] & mask, 0, 0, 0 ])
		}
		else if bits < 16 {
			Address([ self.0[0], self.0[1] & mask, 0, 0 ])
		}
		else if bits < 24 {
			Address([ self.0[0], self.0[1], self.0[2] & mask, 0 ])
		}
		else if bits < 32 {
			Address([ self.0[0], self.0[1], self.0[2], self.0[3] & mask ])
		}
		else if bits == 32 {
			Address(self.0)
		}
		else {
			unreachable!()
		}
	}
	pub fn is_zero(&self) -> bool {
		self.0 == [0,0,0,0]
	}
}
pub struct Interface
{
	local_mac: [u8; 6],
	address: Address,
	mask: u8,
}
impl Interface
{
	pub fn addr(&self) -> Address {
		self.address
	}
}
