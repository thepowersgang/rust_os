
use ::kernel::lib::mem::Box;
use ::kernel::lib::Vec;
use ::kernel::lib::mem::aref::{Aref,ArefBorrow};

/// A feature on a hub port
#[derive(Debug)]
#[derive(Copy,Clone)]
#[repr(u8)]
pub enum PortFeature
{
	Connection  = 0,
	Enable      = 1,
	Suspend     = 2,
	OverCurrent = 3,
	Reset       = 4,
	// - Gap
	Power       = 8,
	LowSpeed    = 9,

	// Change notifications
	CConnection  = 16,
	CEnable      = 17,
	CSuspend     = 18,
	COverCurrent = 19,
	CReset       = 20,
	Test,
	Indicator,
}

pub(crate) fn start_device<'a>(host: super::HostRef, ep0: &'a super::ControlEndpoint, endpoints: Vec<super::Endpoint>) -> crate::device::Instance<'a>
{
	let (int_ep, ) = {
		let mut it = endpoints.into_iter();
		let int_ep = match it.next()
			{
			Some(super::Endpoint::Interrupt(v)) => v,
			_ => panic!("Incorrect endpoints"),
			};
		(int_ep, )
		};

	Box::new(async move {
		let hub_desc = {
			let mut hub_desc_raw = [0; 2+5+32];
			let l = ep0.read_descriptor_raw(0x0129, 0, &mut hub_desc_raw).await.expect("TODO: Error when reading HubDescriptor");
			HubDescriptor::from_bytes(&hub_desc_raw[..l])
			};

		let dev = Aref::new(HubDevice {
			ep0,
			int_ep,

			host,

			ports: {
				let mut v = Vec::new();
				v.resize_with(hub_desc.num_ports as usize, || super::PortState::new());
				v
				},
			hub_desc,
			});
		// 1. Watch for requests to update features?
		// 2. Check for updates on the interrupt endpoint
		loop
		{
			dev.check_interrupt().await;
		}
		})
}

pub(crate) struct HubDevice<'a>
{
	ep0: &'a crate::ControlEndpoint,
	int_ep: crate::InterruptEndpoint,
	hub_desc: HubDescriptor,

	pub host: super::HostRef,
	ports: Vec<super::PortState>,
}
impl HubDevice<'_>
{
	fn static_borrow(self: &Aref<Self>) -> ArefBorrow<HubDevice<'static>> {
		// SAFE: This class doesn't expose `ep0` (which is the borrowed field), so it can't be leaked.
		// SAFE: The Aref structure ensures that the borrow can't be used once Aref is dropped
		unsafe {
			let b: ArefBorrow<Self> = self.borrow();
			::core::mem::transmute(b)
		}
	}

	/// Wait for an interrupt transaction and handle the changes to the ports
	async fn check_interrupt(self: &Aref<Self>)
	{
		let d = self.int_ep.wait().await;
		for i in 0 .. self.hub_desc.num_ports as usize
		{
			let byte_idx = i/8;
			if byte_idx < d.len() && d[byte_idx] & 1 << i%8 != 0
			{
				self.check_port(i).await;
			}
		}
	}

	/// Check for status changes to the given port
	async fn check_port(self: &Aref<Self>, idx: usize)
	{
		// Request the changeset
		let status = self.get_status(idx).await;
		log_notice!("Hub port {}: status={:08x}", idx, status);

		if status & 1 << PortFeature::CConnection as u8 != 0 {
			let connected = status & 1 << PortFeature::Connection as u8 != 0; 
			log_debug!("Conection change: {}", connected);
			self.clear_port_feature(idx, PortFeature::CConnection).await;
			if connected {
				// Newly connected
				// - Hand off to parent's port init code
				let hubref = super::HubRef::Device(self.static_borrow());
				self.ports[idx].signal_connected(hubref, idx as u8);
			}
			else {
				// Disconnected
				todo!("Handle port disconnection");
			}
		}
		if status & 1 << PortFeature::CEnable as u8 != 0 {
			let val = status & 1 << PortFeature::Enable as u8 != 0; 
			log_debug!("Enable change: {}", val);
			self.clear_port_feature(idx, PortFeature::CEnable).await;
		}
		if status & 1 << PortFeature::CSuspend as u8 != 0 {
			let val = status & 1 << PortFeature::Suspend as u8 != 0; 
			log_debug!("Suspend change: {}", val);
			self.clear_port_feature(idx, PortFeature::CSuspend).await;
		}
		if status & 1 << PortFeature::COverCurrent as u8 != 0 {
			let val = status & 1 << PortFeature::OverCurrent as u8 != 0; 
			log_debug!("OverCurrent change: {}", val);
			self.clear_port_feature(idx, PortFeature::COverCurrent).await;
		}
		if status & 1 << PortFeature::CReset as u8 != 0 {
			let val = status & 1 << PortFeature::Reset as u8 != 0; 
			log_debug!("Reset change: {}", val);
			self.clear_port_feature(idx, PortFeature::CReset).await;
		}
	}

	/// Time between setting the `Power` feature and the power being stable
	pub fn power_stable_time_ms(&self) -> u32 {
		self.hub_desc.power_on_to_power_good as u32 * 2
	}

	pub async fn set_port_feature(&self, port_idx: usize, feat: PortFeature) {
		log_debug!("set_port_feature({}, {:?})", port_idx, feat);
		self.ep0.send_request(/*type=*/0x23, /*req_num=*/3/*SET_FEATURE*/, /*value=*/feat as u8 as u16, /*index=*/port_idx as u16, &[]).await;
	}
	pub async fn clear_port_feature(&self, port_idx: usize, feat: PortFeature) {
		log_debug!("clear_port_feature({}, {:?})", port_idx, feat);
		self.ep0.send_request(/*type=*/0x23, /*req_num=*/1/*CLEAR_FEATURE*/, /*value=*/feat as u8 as u16, /*index=*/port_idx as u16, &[]).await;
	}
	pub async fn get_port_feature(&self, port_idx: usize, feat: PortFeature) -> bool {
		log_trace!("get_port_feature({}, {:?})", port_idx, feat);
		let rv = self.get_status(port_idx).await & (1 << feat as u8) != 0;
		log_debug!("get_port_feature({}, {:?}): {}", port_idx, feat, rv);
		rv
	}

	async fn get_status(&self, idx: usize) -> u32
	{
		log_trace!("get_status({})", idx);
		let mut status_raw = [0; 4];
		self.ep0.read_request(/*type=*/0xA3, /*req_num=*/0/*GET_STATUS*/, /*value=*/0, /*port=*/idx as u16, &mut status_raw).await;
		(status_raw[0] as u32) << 0 | (status_raw[1] as u32) << 8 | (status_raw[2] as u32) << 16 | (status_raw[3] as u32) << 24
	}
}

#[derive(Debug)]
#[allow(dead_code)]
struct HubDescriptor
{
	desc_len: u8,
	desc_ty: u8,	// = 0x29

	num_ports: u8,
	/// - `1:0` = Logical Power Switching Mode
	///   - `00` = Ganged (all ports at once)
	///   - `01` = Indivdual ports
	///   - `1X` = Reserved
	/// - `2` = Compound device?
	/// - `4:3` = Over-current protection mode
	///   - `00` = Global (all ports at once)
	///   - `01` = Individual ports
	///   - `1X` = None
	/// - `6:5` = "TT Think Time" (units of 8 FS bit times)
	/// - `7` = Port Indicators Supported
	hub_characteristics: u16,
	/// Time between turning on power and it stabilising, in 2ms intervals
	power_on_to_power_good: u8,
	/// "Maximum current requirements of the Hub Controller electronics in mA."
	hub_control_current: u8,
	device_removable: [u8; 32],
}
impl HubDescriptor
{
	const DESC_TY: u8 = 0x29;
	fn from_bytes(b: &[u8]) -> Self {
		let base_size = 2+5;
		assert!(b.len() >= base_size);
		let mut rv = Self {
			desc_len: b[0],
			desc_ty: b[1],

			num_ports: b[2],
			hub_characteristics: (b[3] as u16) << 0 | (b[4] as u16) << 8,
			power_on_to_power_good: b[5],
			hub_control_current: b[6],
			device_removable: [0; 32],
			};
		if rv.desc_len as usize > b.len() {
			log_error!("Reported descriptor length is longer than buffer");
		}
		if rv.desc_ty != Self::DESC_TY {
			log_error!("Reported descriptor type isn't as expected: {:#x} != exp {:#x}",  rv.desc_ty, Self::DESC_TY);
		}
		let l = ::core::cmp::min(b.len() - base_size, 32);
		rv.device_removable[..l].copy_from_slice(&b[base_size..][..l]);
		rv
	}
}
