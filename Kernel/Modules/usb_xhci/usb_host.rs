
use ::usb_core::host;
use ::usb_core::host::{Handle,EndpointAddr};
use kernel::lib::mem::Box;

mod device0;

pub struct UsbHost
{
    pub(crate) host: crate::HostRef,
}

impl host::HostController for UsbHost
{
	fn init_interrupt(&self, endpoint: EndpointAddr, period_ms: usize, max_packet_size: usize) -> Handle<dyn host::InterruptEndpoint> {
        todo!("");
	}
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::IsochEndpoint> {
		todo!("init_isoch({:?}, max_packet_size={})", endpoint, max_packet_size);
	}
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::ControlEndpoint> {
        if endpoint.dev_addr() == 0 {
            Handle::new(device0::Device0::new(self.host.clone(), max_packet_size))
                .ok().expect("Should fit")
        }
        else {
            todo!("init_control({:?}, max_packet_size={})", endpoint, max_packet_size);
        }
	}
	fn init_bulk_out(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointOut> {
        todo!("");
	}
	fn init_bulk_in(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointIn> {
        todo!("");
	}


	// Root hub maintainence
	fn set_port_feature(&self, port: usize, feature: host::PortFeature) {
        let p = self.host.regs.port(port as u8);
        let (mask,val) = get_feature(feature);
        if mask == 0 { return }
        log_trace!("set_port_feature({},{:?}) {:#x}={:#x}", port, feature, mask, val);
        p.set_sc( (p.sc() & !mask) | val);
	}
	fn clear_port_feature(&self, port: usize, feature: host::PortFeature) {
        let p = self.host.regs.port(port as u8);
        let (mask,_val) = get_feature(feature);
        if mask == 0 { return }
        log_trace!("clear_port_feature({},{:?}) {:#x}", port, feature, mask);
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
            let exp_meta = ::core::ptr::metadata(::core::ptr::null::<Box<ControlEndpoint>>() as *const dyn host::ControlEndpoint);
            let have_meta = ::core::ptr::metadata(hub_endpoint_zero);
            if exp_meta != have_meta {
                log_error!("set_hub_port_speed: Controller passed an endpoint that wasn't our ControlEndpoint - {:?} != exp {:?}",
                    have_meta, exp_meta,
                    );
                return ;
            }
            &**(hub_endpoint_zero as *const _ as *const () as *const Box<ControlEndpoint>)
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
        self.host.set_device_info(Some(&hub_endpoint_zero.device_context), port, speed);
    }
}

struct ControlEndpoint {
    pub(crate) host: crate::HostRef,
    device_context: super::DeviceContextHandle,
}
impl host::ControlEndpoint for ControlEndpoint {
    fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize> {
        todo!("");
    }
	fn in_only<'a>(&'a self, setup_data: &'a [u8], in_buf: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize> {
        todo!("");
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
