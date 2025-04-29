
use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::MacAddr;

mod address;
pub use self::address::Address;
mod rx;
pub(crate) use self::rx::{register_handler,handle_rx_ethernet};
mod nd;
mod icmpv6;
mod headers;
mod routes;
use headers::Ipv6Header;
pub use self::routes::{SelectedRoute,route_lookup, route_enumerate,route_add,route_del};

static INTERFACES: RwLock<Vec<Interface>> = RwLock::new(Vec::new());

// NOTE: Public so it can be passed to RX handler
pub struct Interface
{
	local_mac: MacAddr,
	address: Address,
	mask: u8,
}
impl Interface
{
	pub fn addr(&self) -> Address {
		self.address
	}
}

// NOTE: uses mac address to identify interface
/// Add a new IPv4 interface (address)
pub fn add_interface(local_mac: [u8; 6], address: Address, mask_bits: u8) -> Result<(),()>
{
	let mut lh = INTERFACES.write();
	for interface in lh.iter() {
		if interface.address == address {
			// Whups?
			return Err( () );
		}
	}

	log_info!("Address added: {}/{} on {:x?}", address, mask_bits, local_mac);
	lh.push(Interface {
		local_mac,
		address,
		mask: mask_bits,
		});
	Ok( () )
}

pub async fn send_packet(source: Address, destination: Address, proto: u8, pkt: crate::nic::SparsePacket<'_>)
{
	log_trace!("send_packet({} -> {} 0x{:02x})", source, destination, proto);
	// 1. Look up routing table for destination IP and interface
	let SelectedRoute { source_mac, next_hop, source_ip: _, source_mask } = match route_lookup(source, destination)
		{
		Some(v) => v,
		None => {
			log_notice!("Unable to send to {:?}: No route", destination);
			return	// TODO: Error - No route to host
			},
		};
	// 2. ARP (what if ARP has to wait?)
	// - A wildcard address should resolve to `FF:FF:FF:...`
	//   - Widcard is detected by being a direct address (next = original) and the destination's host part on this interface is all ones
	let dest_mac = if next_hop == destination && destination.mask_host(source_mask) == Address::broadcast().mask_host(source_mask) {
		[0xFF; 6]
	}
	else {
		match nd::resolve(source_mac, next_hop).await
		{
		Some(v) => v,
		None => {
			log_notice!("Unable to send to {:?}: No ARP", destination);
			return
			},	// TODO: Error - No route to host
		}
	};
	// 3. Send
	let hdr = Ipv6Header {
		ver_tc_fl: 0x4000_0000,
		payload_length: pkt.total_len() as u16,
		hop_limit: 255,
		next_header: proto,

		source,
		destination,
		};
	let hdr_bytes = hdr.encode();
	crate::nic::send_from(source_mac, dest_mac, 0x0800, crate::nic::SparsePacket::new_chained(&hdr_bytes, &pkt));
}

pub fn calculate_inner_checksum_rdr(next_header: u8, source: Address, destination: Address, mut reader: crate::nic::PacketReader<'_>) -> u16 {
	let len = reader.remain();
	calculate_inner_checksum_it(next_header, source, destination, (0 .. len).map(|_| reader.read_u8().unwrap()))
}
/// Calculate a checksum, including an IPv6 pseudo-header
/// 
/// NOTE: If the szie hint of `inner` is incorrect, the checksum will be invalid.
pub fn calculate_inner_checksum_it(next_header: u8, source: Address, destination: Address, inner: impl Iterator<Item=u8>) -> u16 {
	let len = inner.size_hint().0 as u32;
	return super::ipv4::calculate_checksum(
		[].iter().copied()
		.chain(source.words().iter().copied())
		.chain(destination.words().iter().copied())
		.chain([
			(len >> 16) as u16,
			(len >> 0) as u16,
			0,
			next_header as u16,
		].iter().copied())
		.chain(Words( inner ))
	);
	struct Words<I>(I);
	impl<I> Iterator for Words<I>
	where I: Iterator<Item=u8>
	{
		type Item = u16;
		
		fn next(&mut self) -> Option<Self::Item> {
			// NOTE: This only really works on fused iterators
			match (self.0.next(),self.0.next()) {
			(Some(a),Some(b)) => Some(u16::from_be_bytes([a,b])),
			(Some(a),None) => Some(u16::from_be_bytes([a,0])),
			(None,_) => None,
			}
		}
	}
}