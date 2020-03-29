// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/arp.rs
//! "Address Resolution Protocol"
use kernel::sync::RwLock;
use kernel::lib::VecMap;
use crate::nic::MacAddr;

static CACHE: RwLock<VecMap<crate::ipv4::Address, Option<MacAddr>>> = RwLock::new(VecMap::new_const());

pub fn handle_packet(_physical_interface: &dyn crate::nic::Interface, _source_mac: [u8; 6], mut r: crate::nic::PacketReader)
{
	// TODO: Length test
	let hw_ty  = r.read_u16n().unwrap();
	let sw_ty  = r.read_u16n().unwrap();
	let hwsize = r.read_u8().unwrap();
	let swsize = r.read_u8().unwrap();
	let code = r.read_u16n().unwrap();
	log_debug!("ARP HW {:04x} {}B SW {:04x} {}B req={}", hw_ty, hwsize, sw_ty, swsize, code);
	if hwsize == 6 {
		let mac = {
			let mut b = [0; 6];
			r.read(&mut b).unwrap();
			b
			};
		log_debug!("ARP HW {:?}", ::kernel::logging::HexDump(&mac));
	}
	if swsize == 4 {
		let ip = {
			let mut b = [0; 4];
			r.read(&mut b).unwrap();
			b
			};
		log_debug!("ARP SW {:?}", ip);
	}
}

pub fn peek_v4(mac: MacAddr, ip: crate::ipv4::Address)
{
	if CACHE.read().get(&ip).is_none()
	{
		let mut lh = CACHE.write();
		lh.insert(ip, Some(mac));
	}
}

pub fn lookup_v4(addr: crate::ipv4::Address) -> Option<MacAddr>
{
	match CACHE.read().get(&addr)
	{
	Some(Some(v)) => return Some(*v),
	Some(None) => {},
	None => {},
	}
	todo!("ARP request {}", addr);
	// - Send request packet
	// - Wait until the cache has the requested host in it (with timeout)
}
