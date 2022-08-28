
use ::usb_core::host;
use ::usb_core::host::{Handle,EndpointAddr};
use kernel::lib::mem::Box;

/// Device 0 (handles enumeration)
mod device0;
/// Endpoint 0 (special control endpoint)
mod control;
mod bulk;
mod interrupt;

pub struct UsbHost
{
	pub(crate) host: crate::HostRef,
}

fn type_name_of<T>(_: &T) -> &'static str { ::core::any::type_name::<T>() }
macro_rules! make_handle_assert {
	($v:expr) => {
		match Handle::new($v) {
		Ok(v) => v,
		Err(v) => panic!("{} didn't fit - {} > {}", type_name_of(&v), ::core::mem::size_of_val(&v), ::core::mem::size_of::<Handle<dyn host::InterruptEndpoint>>() - ::core::mem::size_of::<usize>()),
		}
	}
}

impl host::HostController for UsbHost
{
	fn init_interrupt(&self, endpoint: EndpointAddr, period_ms: usize, max_packet_size: usize) -> Handle<dyn host::InterruptEndpoint> {
		// Boxed, becuase it has a bunch of extra storage
		make_handle_assert!( Box::new(interrupt::Interrupt::new(self.host.clone(), endpoint, period_ms, max_packet_size).expect("Interrupt")) )
	}
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::IsochEndpoint> {
		todo!("init_isoch({:?}, max_packet_size={})", endpoint, max_packet_size);
	}
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::ControlEndpoint> {
		if endpoint.dev_addr() == 0 {
			// Device 0 is special
			assert!( endpoint.endpt() == 0, "Creating control endpoint for device 0 not on endpoint 0" );
			make_handle_assert!( device0::Device0::new(self.host.clone(), max_packet_size) )
		}
		else if endpoint.endpt() == 0 {
			// Endpoint 0 needs logic to monitor for configuration changes
			make_handle_assert!( control::Endpoint0::new(self.host.clone(), endpoint.dev_addr(), max_packet_size).expect("Endpoint0"))
		}
		else {
			make_handle_assert!( control::Control::new(self.host.clone(), endpoint.dev_addr(), endpoint.endpt(), max_packet_size).expect("Control") )
		}
	}
	fn init_bulk_out(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointOut> {
		make_handle_assert!(bulk::BulkOut::new(self.host.clone(), endpoint.dev_addr(), endpoint.endpt(), max_packet_size).expect("BulkOut"))
	}
	fn init_bulk_in(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointIn> {
		make_handle_assert!(bulk::BulkIn::new(self.host.clone(), endpoint.dev_addr(), endpoint.endpt(), max_packet_size).expect("BulkIn"))
	}


	// Root hub maintainence
	fn set_port_feature(&self, port: usize, feature: host::PortFeature) {
		let p = self.host.regs.port(port as u8);
		let (mask,val) = get_feature(feature);
		if mask == 0 { return }
		log_trace!("set_port_feature({}, {:?}): {:#x}={:#x}", port, feature, mask, val);

		p.set_sc( (p.sc() & !mask) | val);
	}
	fn clear_port_feature(&self, port: usize, feature: host::PortFeature) {
		let p = self.host.regs.port(port as u8);
		let (mask,_val) = get_feature(feature);
		if mask == 0 { return }
		log_trace!("clear_port_feature({}, {:?}): {:#x}", port, feature, mask);
		p.set_sc(p.sc() & !mask);
	}
	fn get_port_feature(&self, port: usize, feature: host::PortFeature) -> bool {
		let p = self.host.regs.port(port as u8);
		let (mask,val) = get_feature(feature);
		if mask == 0 { return false }
		let rv = p.sc() & mask == val;
		log_trace!("get_port_feature({}, {:?}): {} ({:#x}=={:#x})",  port, feature, rv, mask, val);
		if let host::PortFeature::Enable = feature {
			if rv {
				self.host.set_device_info(None, port, (p.sc() >> 10) as u8 & 0xF);
			}
		}
		rv
	}

	fn async_wait_root(&self) -> host::AsyncWaitRoot {
		struct AsyncWaitRoot {
			host: super::HostRef,
		}
		impl core::future::Future for AsyncWaitRoot {
			type Output = usize;
			fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
				// Register for wake first
				*self.host.port_update_waker.lock() = cx.waker().clone();
				// Then check if there's a bit available
				if let Some(idx) = self.host.port_update.get_first_set_and_clear() {
					log_debug!("Port update: {}", idx);
					//*self.host.port_update_waker.lock() = 
					return core::task::Poll::Ready(idx);
				}
				core::task::Poll::Pending
			}
		}
		usb_core::host::AsyncWaitRoot::new(AsyncWaitRoot {
			host: self.host.reborrow(),
			}).ok().expect("Over-size task in `async_wait_root`")
	}

	fn set_hub_port_speed(&self, hub_endpoint_zero: &dyn host::ControlEndpoint, port: usize, speed: host::HubPortSpeed) {
		// HACK TIME! Use the pointer metadata for `Any` hackery.
		// SAFE: Only uses the cast when the metadata matches (meaing that it's the same type)
		let hub_endpoint_zero = unsafe {
			let exp_meta = ::core::ptr::metadata(::core::ptr::null::<Box<control::Control>>() as *const dyn host::ControlEndpoint);
			let have_meta = ::core::ptr::metadata(hub_endpoint_zero);
			if exp_meta != have_meta {
				log_error!("set_hub_port_speed: Controller passed an endpoint that wasn't our ControlEndpoint - {:?} != exp {:?}",
					have_meta, exp_meta,
					);
				return ;
			}
			&**(hub_endpoint_zero as *const _ as *const () as *const Box<control::Control>)
			};
		// Get the route string of the parent hub, and tack this port onto it
		let speed = match speed
			{
			::usb_core::host::HubPortSpeed::Full => 1,
			::usb_core::host::HubPortSpeed::Low => 2,
			::usb_core::host::HubPortSpeed::High => 3,
			::usb_core::host::HubPortSpeed::Super => 4,
			::usb_core::host::HubPortSpeed::SuperSpeedPlusG2 => 5,
			::usb_core::host::HubPortSpeed::SuperSpeedPlusG1X2 => 6,
			::usb_core::host::HubPortSpeed::SuperSpeedPlusG2X2 => 7,
			};
		self.host.set_device_info(Some(hub_endpoint_zero.addr), port, speed);
	}
}

fn get_feature(feature: host::PortFeature) -> (u32, u32) {
	fn bit(i: usize) -> (u32, u32) {
		(1 << i, 1 << i)
	}
	fn none() -> (u32,u32) {
		(0,0)
	}
	match feature
	{
	host::PortFeature::Connection => bit(0),
	host::PortFeature::Enable   => bit(1),
	host::PortFeature::Suspend  => none(),
	host::PortFeature::OverCurrent  => bit(3),
	host::PortFeature::Reset  => bit(4),
	host::PortFeature::Power  => bit(9),
	host::PortFeature::LowSpeed => none(),//(0xF << 10, 0),  // TODO
	host::PortFeature::CConnection => bit(17),
	host::PortFeature::CEnable => bit(18),
	host::PortFeature::CSuspend => none(),
	host::PortFeature::COverCurrent => bit(20),
	host::PortFeature::CReset => bit(21),
	host::PortFeature::Test => none(),
	host::PortFeature::Indicator => (3 << 14, 2 << 14),
	}
}

/// Create an `AsyncWaitIo` instance (boxes if required)
fn make_asyncwaitio<'a, T>(f: impl ::core::future::Future<Output=T> + Send + Sync + 'a) -> host::AsyncWaitIo<'a, T> {
	host::AsyncWaitIo::new(f)
		.unwrap_or_else(|v| host::AsyncWaitIo::new(
			::kernel::lib::mem::boxed::Box::pin(v)).ok().unwrap()
			)
}


fn iter_contigious_phys(data: &[u8]) -> impl Iterator<Item=(u64, u16, bool)> + '_ {
	struct V<'a> {
		data: &'a [u8],
		remain: usize,
		ofs: usize,
	}
	impl<'a> ::core::iter::Iterator for V<'a> {
		type Item = (u64, u16, bool);
		fn next(&mut self) -> Option<Self::Item> {
			use ::kernel::memory::virt::get_phys;
			assert!(self.ofs <= self.data.len(), "{}+{} > {}", self.ofs, self.remain, self.data.len());
			if self.ofs == self.data.len() {
				return None;
			}

			while self.ofs+self.remain < self.data.len() && get_phys(&self.data[self.ofs+self.remain]) == get_phys(self.data.as_ptr()) + self.remain as u64 {
				if self.ofs+self.remain + 0x1000 < self.data.len() {
					self.remain = self.data.len() - self.ofs;
				}
				else {
					self.remain += 0x1000;
				}
			}
			let is_last = self.ofs + self.remain == self.data.len();
			let rv = (get_phys(&self.data[self.ofs]), self.remain as _, is_last,);
			self.ofs += self.remain;
			self.remain = 0;
			Some(rv)
		}
	}
	V {
		data,
		ofs: 0,
		remain: usize::min(0x1000 - (data.as_ptr() as usize & 0xFFF), data.len() ),
	}
}
