// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/ipv4.rs
//! IPv4 (Layer 3)
use kernel::lib::Vec;
use kernel::sync::RwLock;

// List of protocol numbers and handlers
static PROTOCOLS: RwLock<Vec<(u8, ProtoHandler)>> = RwLock::new(Vec::new_const());
static INTERFACES: RwLock<Vec<Interface>> = RwLock::new(Vec::new_const());

pub fn register_handler(proto: u8, handler: fn(&Interface, Address, ::nic::PacketReader)) -> Result<(), ()>
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
pub fn handle_rx_ethernet(_physical_interface: &::nic::Interface, _source_mac: [u8; 6], mut reader: ::nic::PacketReader) -> Result<(), ()>
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
		return Err( () );
	}
	let hdr_len = hdr.get_header_length();
	if hdr_len < pre_header_reader.remain()
	{
		// Malformed packet, header size too small
		return Err( () );
	}
	
	while reader.remain() > pre_header_reader.remain() - hdr_len
	{
		// TODO: options!
		match reader.read_u8()?
		{
		_ => {},
		}
	}
	
	// Validate checksum: Sum all of the bytes
	{
		let mut reader = pre_header_reader;
		let mut sum = 0;
		for _ in 0 .. hdr_len / 2
		{
			sum += reader.read_u16n().unwrap() as usize;
		}
		while sum > 0xFFFF
		{
			sum = (sum & 0xFFFF) + (sum >> 16);
		}
		if sum != 0 {
			log_warning!("IP Checksum failure, sum is {:#x}", sum);
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

	
	// TODO: Check destination IP against known interfaces.
	// - Could also be doing routing.
	for interface in INTERFACES.read().iter()
	{
		// TODO: Interfaces should be locked to the physical interface too
		if interface.address == hdr.destination
		{
			// TODO: Should there be per-interface handlers?

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
			// No handler, but the interface is known
			return Ok( () );
		}
	}
	//else
	{
		// Routing.
	}
	
	Ok( () )
}

pub fn send_packet(source: Address, dest: Address, pkt: ::nic::SparsePacket)
{
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
	fn read(reader: &mut ::nic::PacketReader) -> Result<Self, ()>
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
	fn set_has_more_fragments(&mut self) {
		self.flags |= 1 << 5;
	}

	fn get_fragment_ofs(&self) -> usize {
		((self.frag_ofs_high as usize) << 5) | (self.flags & 0x1F) as usize
	}
}

enum ProtoHandler
{
	/// Direct in-kernel handling (e.g. TCP)
	DirectKernel(fn(&Interface, Address, ::nic::PacketReader)),
	/// Indirect user handling (pushes onto a buffer for the user to read from)
	// Ooh, another use for stack_dst, a DST queue!
	User(Address, ()),
}
impl ProtoHandler
{
	fn dispatch(&self, i: &Interface, src: Address, _dest: Address, r: ::nic::PacketReader)
	{
		match *self
		{
		ProtoHandler::DirectKernel(fcn) => fcn(i, src, r),
		ProtoHandler::User(..) => todo!("User-bound raw IP connections"),
		}
	}
}

#[derive(Copy,Clone,Default,PartialEq,PartialOrd,Eq,Ord,Debug)]
pub struct Address([u8; 4]);
impl ::core::fmt::Display for Address
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
	}
}
pub struct Interface
{
	address: Address,
}
impl Interface
{
	pub fn addr(&self) -> Address {
		self.address
	}
}
