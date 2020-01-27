//
//
//
//! USB Device driver interface
use ::kernel::sync::Mutex;
use ::kernel::lib::mem::Box;
use ::kernel::lib::Vec;

#[derive(PartialOrd,PartialEq,Copy,Clone)]
pub enum MatchLevel
{
	None,
	Generic,	// Matches the class

	Vendor,	// Vendor-specific for the class
	Precise,	// Matched on VID/DID
}

pub type Instance<'a> = Box<dyn ::core::future::Future<Output=()> + Send + Sync + 'a>;

/// Driver for an interface
pub trait Driver: Sync
{
	fn name(&self) -> &str;
	fn matches(&self, vendor_id: u16, device_id: u16, class_code: u32) -> MatchLevel;
	fn start_device<'a>(&self, ep0: &'a super::ControlEndpoint, endpoints: Vec<super::Endpoint>, descriptors: &[u8]) -> Instance<'a>;
}

static S_DRIVERS: Mutex<Vec<&'static dyn Driver>> = Mutex::new(Vec::new_const());

/// Register a new driver
pub fn register_driver(ptr: &'static dyn Driver)
{
	S_DRIVERS.lock().push(ptr);
}

/// Locate a driver matching the given device
pub(crate) fn find_driver(vendor_id: u16, device_id: u16, class_code: u32) -> Option<&'static dyn Driver>
{
	let lh = S_DRIVERS.lock();
	let mut best = None;
	for &d in lh.iter()
	{
		let ml = d.matches(vendor_id, device_id, class_code);
		if let MatchLevel::None = ml {
			continue;
		}
		match best
		{
		None => {
			best = Some( (ml, d) );
			},
		Some( (oml, _) ) => {
			if oml < ml {
				best = Some( (ml, d) );
			}
			},
		}
	}
	best.map(|v| v.1)
}

