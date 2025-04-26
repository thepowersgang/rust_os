use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::PacketReader;
use crate::nic::MacAddr;
use super::{Address,Ipv6Header};
use super::{Interface,INTERFACES};

static PROTOCOL_HANDLDERS: RwLock<Vec<(u8, ProtoHandler)>> = RwLock::new(Vec::new());

pub fn handle_rx_ethernet(_physical_interface: &dyn crate::nic::Interface, iface_mac: MacAddr, source_mac: MacAddr, mut reader: PacketReader) -> Result<(), ()>
{
	let hdr = match Ipv6Header::read(&mut reader)
		{
		Ok(v) => v,
		Err(_) => {
			log_warning!("Undersized packet: Ran out of data reading header");
			return Err( () );
			},
		};
	if hdr.ver_tc_fl >> 28 != 6 {
		log_warning!("Malformed packet: version isn't 6 - ver_tc_fl={:08x}", hdr.ver_tc_fl);
		return Err( () );
	}

	// Apply routing (look up a matching interface)

	// Check extension headers
	let mut next_header = hdr.next_header;
	loop {
		next_header = match hdr.next_header {
			0 => todo!("Hop-by-hop options"),
			43 => todo!("Routing Header"),
			44 => todo!("Fragment Header"),
			60 => {
				let next = reader.read_u8()?;
				let len = reader.read_u8()? as usize * 8;
				for _ in 0 .. len {
					reader.read_u8()?;
				}
				next
			},
			59 => return Ok(()),
			_ => break,
			};
	}

	// Check destination IP against known interfaces.
	// - Could also be doing routing.
	for interface in INTERFACES.read().iter()
	{
		// TODO: Interfaces should be locked to the physical interface too
		if interface.local_mac == iface_mac && (interface.address == hdr.destination || hdr.destination == Address::broadcast())
		{
			if hdr.source.mask_net(interface.mask) == interface.address.mask_net(interface.mask) {
				// Snoop the source MAC into the neighbour-discovery cache
				super::nd::learn(iface_mac, source_mac, hdr.source);
			}

			// TODO: ICMPv6 handling
			// - Needs to include pings and status replies
			if next_header == 58 {
				// ICMPv6 (includes ND, type 133)
			}

			// Figure out which sub-protocol to send this packet to
			// - Should there be alternate handlers for 
			for &(id,ref handler) in PROTOCOL_HANDLDERS.read().iter()
			{
				if id == next_header
				{
					handler.dispatch(interface, hdr.source, hdr.destination, reader);
					return Ok( () );
				}
			}
			log_debug!("Unknown protocol {}", next_header);
			return Ok( () );
		}
	}
	Ok( () )
}

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