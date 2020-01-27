
use ::kernel::lib::mem::Box;
use ::kernel::lib::Vec;
use ::kernel::lib::mem::aref::{Aref};


#[derive(Debug)]
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

		let mut dev = HubDevice {
			ep0,
			int_ep,
			parent: Aref::new(super::HubDevice::new(host, hub_desc.num_ports as usize)),
			hub_desc,
			};
		// 1. Watch for requests to update features?
		// 2. Check for updates on the interrupt endpoint
		loop
		{
			dev.check_interrupt().await;
		}
		})
}

struct HubDevice<'a>
{
	ep0: &'a crate::ControlEndpoint,
	int_ep: crate::InterruptEndpoint,
	hub_desc: HubDescriptor,

	parent: Aref<super::HubDevice>,
}
impl HubDevice<'_>
{
	async fn check_interrupt(&mut self)
	{
		let d = self.int_ep.wait().await;
		for i in 0 .. self.hub_desc.num_ports as usize
		{
			let byte_idx = i/8;
			if byte_idx < d.len() && d[byte_idx] & 1 << i%8 != 0
			{
				log_notice!("Port change: {}", i);
				self.check_port(i).await;
			}
		}
	}

	async fn check_port(&self, idx: usize)
	{
		// Request the changeset
		let status = {
			let mut status_raw = [0; 4];
			self.ep0.read_request(/*type=*/0xA3, /*req_num=*/0/*GET_STATUS*/, /*value=*/0, /*port=*/idx as u16, &mut status_raw).await;
			(status_raw[0] as u32) << 0 | (status_raw[1] as u32) << 8 | (status_raw[2] as u32) << 16 | (status_raw[3] as u32) << 24
			};
		log_debug!("port {}: status={:08x}", idx, status);
		if status & 1 << PortFeature::CConnection as u8 != 0 {
			if status & 1 << PortFeature::Connection as u8 != 0 {
				// Newly connected
				// - Hand off to parent's port init code
				super::HubDevice::port_connected(self.parent.borrow(), idx);
			}
			else {
				// Disconnected
				super::HubDevice::port_disconnected(self.parent.borrow(), idx);
			}
		}
	}
}

#[derive(Debug)]
struct HubDescriptor
{
	desc_len: u8,
	desc_ty: u8,

	num_ports: u8,
	hub_characteristics: u16,
	power_on_to_power_good: u8,	// 2ms intervals
	hub_control_current: u8,	// Max controllable current
	device_removable: [u8; 32],
}
impl HubDescriptor
{
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
		let l = ::core::cmp::min(b.len() - base_size, 32);
		rv.device_removable[..l].copy_from_slice(&b[base_size..][..l]);
		rv
	}
}
