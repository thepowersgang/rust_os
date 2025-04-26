/// IPv6 Neighbour Discovery

use kernel::sync::RwLock;
use kernel::lib::VecMap;
use crate::nic::MacAddr;

static CACHE: RwLock<VecMap<(MacAddr,super::Address), Option<MacAddr>>> = RwLock::new(VecMap::new());

pub async fn resolve(source_mac: crate::nic::MacAddr, next_hop: super::Address) -> Option<crate::nic::MacAddr> {
	match CACHE.read().get(&(source_mac,next_hop)) {
	Some(Some(v)) => return Some(*v),
	_ => {},
	}
	todo!("IPv6 ND lookup")
}
pub fn learn(iface_mac: crate::nic::MacAddr, source_mac: crate::nic::MacAddr, addr: super::Address) {
	let mut lh = CACHE.write();
	// TODO: Error checking?
	lh.insert((iface_mac,addr), Some(source_mac));
}