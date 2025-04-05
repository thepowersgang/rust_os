
use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::MacAddr;
use super::Address;

/// Network routes
static ROUTES: RwLock<Vec<Route>> = RwLock::new(Vec::new());

#[derive(Copy, Clone, PartialEq)]
pub struct Route
{
	pub network: Address,
	pub mask: u8,
	pub next_hop: Address,
}

pub fn route_add(route: Route) {
	match route_lookup(Address::zero(), route.next_hop) {
	Some(sr) if sr.next_hop == route.next_hop => {},
	_ => {
		// TODO: Error
	}
	}

	let mut lh = ROUTES.write();
	for r in lh.iter_mut() {
		// TODO: Check for duplicates
		if *r == route {
			return ;
		}
	}
	lh.push(route);
}
pub fn route_del(route: Route) -> bool {
	let mut lh = ROUTES.write();
	let orig_s = lh.len();
	lh.retain(|r| *r != route);
	lh.len() < orig_s
}
pub fn route_enumerate(index: usize) -> (usize, Option<Route>) {
	let lh = ROUTES.read();
	(lh.len(), lh.get(index).copied(),)
}

/// Return value for [route_lookup]
pub struct SelectedRoute {
	pub source_ip: Address,
	pub source_mac: MacAddr,
	pub next_hop: Address,
}
/// Determine the source IP/MAC and NextHop for a given combination of soure IP (could be `0.0.0.0`) and destination address
pub fn route_lookup(source: Address, dest: Address) -> Option<SelectedRoute>
{
	let mut best: Option<(u8, Address)> = None;
	// Check static route list
	for route in ROUTES.read().iter()
	{
		if route.network.mask(route.mask) == dest.mask(route.mask) {
			match best {
			Some((m, _)) if m >= route.mask => {},
			_ => {
				best = Some((route.mask, route.next_hop));
			}
			}
		}
	}

	let mut src_for_best: Option<(Address, MacAddr)> = None;
	for interface in super::INTERFACES.read().iter()
	{
		if source.is_zero() || interface.address == source {
			if let Some((_, a)) = best {
				if interface.address.mask(interface.mask) == a.mask(interface.mask) {
					src_for_best = Some((interface.address, interface.local_mac));
				}
			}
			// On-link?
			if interface.address.mask(interface.mask) == dest.mask(interface.mask) {
				// Immedidately return if this interface is more specific than the best non-interface route
				if best.map_or(true, |(m, _)| m < interface.mask ) {
					return Some(SelectedRoute { source_ip: interface.address, source_mac: interface.local_mac, next_hop: dest});
				}
			}
		}
	}

	if let Some((_, gw)) = best {
		if let Some((local_addr, mac)) = src_for_best {
			return Some(SelectedRoute { source_ip: local_addr, source_mac: mac, next_hop: gw});
		}
		else {
		}
	}

	None
}