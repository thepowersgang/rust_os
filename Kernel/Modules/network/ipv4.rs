// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/ipv4.rs
//! IPv4 (Layer 3)
use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::MacAddr;

mod address;
pub use self::address::Address;

mod headers;
use self::headers::Ipv4Header;

mod routes;
pub use self::routes::{SelectedRoute, Route, route_lookup, route_add, route_del, route_enumerate};

mod rx;
pub use self::rx::{handle_rx_ethernet, register_handler};

/// Active IPv4 interfaces
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
/// Remove an existing IPv4 interface/address
pub fn del_interface(local_mac: [u8; 6], address: Address, mask_bits: u8) -> Result<(),()>
{
	let mut found = false;
	let mut lh = INTERFACES.write();
	lh.retain(|iface| {
		if iface.local_mac == local_mac && iface.address == address && iface.mask == mask_bits {
			found = true;
			false
		}
		else {
			true
		}
	});
	if found {
		Ok( () )
	}
	else {
		Err( () )
	}
}

#[cfg(any())]
pub fn listen_raw(local_addr: Address, proto: u8, remote_mask: (Address, u8)) -> RawListenHandle
{
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

/// Send a raw packet
pub async fn send_packet(source: Address, dest: Address, proto: u8, pkt: crate::nic::SparsePacket<'_>) -> Result<(),()>
{
	log_trace!("send_packet({:?} -> {:?} 0x{:02x})", source, dest, proto);
	// 1. Look up routing table for destination IP and interface
	let SelectedRoute { source_mac, next_hop, source_ip: _, source_mask } = match route_lookup(source, dest)
		{
		Some(v) => v,
		None => {
			log_notice!("Unable to send to {:?}: No route", dest);
			return Err(());
			},
		};
	// 2. ARP (what if ARP has to wait?)
	// - A wildcard address should resolve to `FF:FF:FF:...`
	//   - Widcard is detected by being a direct address (next = original) and the destination's host part on this interface is all ones
	let dest_mac = if next_hop == dest && dest.mask_host(source_mask) == Address([0xFF;4]).mask_host(source_mask) {
		[0xFF; 6]
	}
	else {
		match crate::arp::lookup_v4(source_mac, next_hop).await
		{
		Some(v) => v,
		None => {
			log_notice!("Unable to send to {:?}: No ARP", dest);
			return Err(());
			},
		}
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
	crate::nic::send_from(source_mac, dest_mac, 0x0800, crate::nic::SparsePacket::new_chained(&hdr_bytes, &pkt));
	Ok( () )
}
