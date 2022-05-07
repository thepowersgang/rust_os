// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/arp.rs
//! "Address Resolution Protocol"
use kernel::sync::RwLock;
use kernel::lib::VecMap;
use crate::nic::MacAddr;

static CACHE: RwLock<VecMap<crate::ipv4::Address, Option<MacAddr>>> = RwLock::new(VecMap::new_const());
static SLEEPERS: ::kernel::futures::Condvar = ::kernel::futures::Condvar::new();

pub fn handle_packet(_physical_interface: &dyn crate::nic::Interface, _source_mac: [u8; 6], mut r: crate::nic::PacketReader)
{
	// TODO: Length test
	let hw_ty  = r.read_u16n().unwrap();
	let sw_ty  = r.read_u16n().unwrap();
	let hwsize = r.read_u8().unwrap();
	let swsize = r.read_u8().unwrap();
	let code = r.read_u16n().unwrap();
	log_debug!("ARP HW {:04x} {}B SW {:04x} {}B req={}", hw_ty, hwsize, sw_ty, swsize, code);
	let hwaddr = match hwsize
		{
		6 => {
			let mac = {
				let mut b = [0; 6];
				r.read(&mut b).unwrap();
				b
				};
			log_debug!("ARP HW {:?}", ::kernel::logging::HexDump(&mac));
			mac
			},
		_ => return,
		};
	let swaddr = match swsize
		{
		4 => {
			let ip = {
				let mut b = [0; 4];
				r.read(&mut b).unwrap();
				b
				};
			log_debug!("ARP SW {:?}", ip);
			crate::ipv4::Address(ip)
			},
		_ => return,
		};
	
	snoop_v4(hwaddr, swaddr);
}

/// Inform the ARP layer of an observed mapping
pub fn snoop_v4(mac: MacAddr, ip: crate::ipv4::Address)
{
	if CACHE.read().get(&ip).is_none()
	{
		let mut lh = CACHE.write();
		log_debug!("ARP snoop: {:?} = {:x?}", ip, mac);
		lh.insert(ip, Some(mac));
		SLEEPERS.wake_all();
	}
	else {
		// If the IP changes, then there's something funny here.
	}
}

/// Acquire a MAC address for the given IP
pub async fn lookup_v4(interface_mac: crate::nic::MacAddr, addr: crate::ipv4::Address) -> Option<MacAddr>
{
	match CACHE.read().get(&addr)
	{
	Some(Some(v)) => return Some(*v),
	Some(None) => {},
	None => {},
	}
	log_debug!("Sending ARP request for {} from {:?}", addr, interface_mac);
	// - Send request packet
	let dest_mac = [0xFF; 6];
	let request = [
		0x00,0x01,	// Ethernet
		0x08,0x00,	// IPv4
		6, 4,	// hwsize, swsize
		0, 1,	// operation
		interface_mac[0], interface_mac[1], interface_mac[2], interface_mac[3], interface_mac[4], interface_mac[5],
		0,0,0,0,
		0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,
		addr.0[0], addr.0[1], addr.0[2], addr.0[3],
		];
	crate::nic::send_from(interface_mac, dest_mac, 0x0806, crate::nic::SparsePacket::new_root(&request));

	// - Wait until the cache has the requested host in it (with timeout)
	const TIMEOUT_MS: u64 = 1000;
	let timeout_time = ::kernel::time::ticks() + TIMEOUT_MS;
	loop
	{
		// Get condvar key, then check if the IP is present, THEN wait until the key changes
		let key = SLEEPERS.get_key();
		match CACHE.read().get(&addr)
		{
		Some(Some(v)) => return Some(*v),
		_ => {},
		}
		// Sleep up to the timeout.
		let sleep_duration = match timeout_time.checked_sub(::kernel::time::ticks())
			{
			None => return None,
			Some(v) => v,
			};
		::kernel::futures::join_one(
			SLEEPERS.wait(key),
			::kernel::futures::msleep(sleep_duration as usize)
			).await;
	}
}
