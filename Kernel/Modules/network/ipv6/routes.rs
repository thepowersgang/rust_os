
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

pub fn route_add(route: Route) -> Result<(),()> {
	match route_lookup(Address::zero(), route.next_hop) {
	Some(sr) if sr.next_hop == route.next_hop => {},
	_ => {
		log_error!("Malformed route: No interface can directly reach next-hop {}", route.next_hop);
		return Err(());
	}
	}

	let mut lh = ROUTES.write();
	for r in lh.iter_mut() {
		if *r == route {
			log_warning!("Duplicate route");
			return Err(());
		}
	}
	log_info!("Route added: {}/{} via {}", route.network, route.network, route.next_hop);
	lh.push(route);
	Ok(())
}
pub fn route_del(route: Route) -> Result<(),()> {
	let mut lh = ROUTES.write();
	let orig_s = lh.len();
	lh.retain(|r| *r != route);
	match lh.len() < orig_s
	{
	true => Ok(()),
	false => Err(()),
	}
}
pub fn route_enumerate(index: usize) -> (usize, Option<Route>) {
	let lh = ROUTES.read();
	(lh.len(), lh.get(index).copied(),)
}

/// Return value for [route_lookup]
pub struct SelectedRoute {
	pub source_ip: Address,
	pub source_mac: MacAddr,
	pub source_mask: u8,
	pub next_hop: Address,
}
/// Determine the source IP/MAC and NextHop for a given combination of soure IP (could be `0.0.0.0`) and destination address
pub fn route_lookup(source: Address, dest: Address) -> Option<SelectedRoute>
{
	let mut best: Option<(u8, Address)> = None;
	// Check static route list
	for route in ROUTES.read().iter()
	{
		if route.network.mask_net(route.mask) == dest.mask_net(route.mask) {
			match best {
			Some((m, _)) if m >= route.mask => {},
			_ => {
				best = Some((route.mask, route.next_hop));
			}
			}
		}
	}

	struct SourceInfo {
		addr: Address,
		mask: u8,
		mac: MacAddr,
	}
	impl SourceInfo {
		fn from_iface(interface: &super::Interface) -> Self {
			SourceInfo { addr: interface.address, mask: interface.mask, mac: interface.local_mac }
		}
		fn to_rv(self, next_hop: Address) -> SelectedRoute {
			SelectedRoute { source_ip: self.addr, source_mac: self.mac, source_mask: self.mask, next_hop }
		}
	}
	let mut src_for_best: Option<SourceInfo> = None;
	for interface in super::INTERFACES.read().iter()
	{
		let si = SourceInfo::from_iface(interface);
		if source == Address::zero() || interface.address == source {
			// TODO: Special case `255.255.255.255` to send out the source interface
			if dest == Address::broadcast() {
				return Some(si.to_rv(dest));
			}
			
			// On-link?
			if interface.address.mask_net(interface.mask) == dest.mask_net(interface.mask) {
				// Immedidately return if this interface is more specific than the best non-interface route
				if best.map_or(true, |(m, _)| m < interface.mask ) {
					return Some(si.to_rv(dest));
				}
			}
			if let Some((_, a)) = best {
				if interface.address.mask_net(interface.mask) == a.mask_net(interface.mask) {
					src_for_best = Some(si);
				}
			}
		}
	}

	if let Some((_, gw)) = best {
		if let Some(si) = src_for_best {
			return Some(si.to_rv(gw));
		}
		else {
		}
	}

	None
}