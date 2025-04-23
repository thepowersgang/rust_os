
use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::PacketReader;

use super::{Address,Interface};
use super::Ipv4Header;
use super::calculate_checksum;

use super::INTERFACES;

/// List of protocol numbers and handlers
static PROTOCOL_HANDLDERS: RwLock<Vec<(u8, ProtoHandler)>> = RwLock::new(Vec::new());

/// Register a protocol handler with this layer
pub fn register_handler(proto: u8, handler: fn(&Interface, Address, PacketReader)) -> Result<(), ()>
{
	let mut lh = PROTOCOL_HANDLDERS.write();
	for &(p, _) in lh.iter()
	{
		if p == proto {
			return Err( () );
		}
	}
	lh.push( (proto, ProtoHandler::DirectKernel(handler),) );
	Ok( () )
}

/// Handle an incoming packet
pub fn handle_rx_ethernet(_physical_interface: &dyn crate::nic::Interface, iface_mac: [u8; 6], source_mac: [u8; 6], mut reader: PacketReader) -> Result<(), ()>
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
		// - I.e. the remaining byte count equals the number of bytes after the header 
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
		if interface.local_mac == iface_mac && (interface.address == hdr.destination || hdr.destination.0 == [0xFF; 4])
		{
			// TODO: Should there be per-interface handlers?

			// TODO: Check if the source address is from the same subnet, and only cache in ARP if it is
			crate::arp::snoop_v4(source_mac, hdr.source);

			// TODO: Support raw socket Rx

			// Figure out which sub-protocol to send this packet to
			// - Should there be alternate handlers for 
			for &(id,ref handler) in PROTOCOL_HANDLDERS.read().iter()
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