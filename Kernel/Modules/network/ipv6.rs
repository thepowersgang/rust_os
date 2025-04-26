
use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::MacAddr;

mod address;
pub use self::address::Address;
mod rx;
pub(crate) use self::rx::{register_handler,handle_rx_ethernet};
mod nd;
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