
use ::usb_core::host;
use ::usb_core::host::{Handle,EndpointAddr};

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
            todo!("");
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
        todo!("");
	}
	fn clear_port_feature(&self, port: usize, feature: host::PortFeature) {
        todo!("");
	}
	fn get_port_feature(&self, port: usize, feature: host::PortFeature) -> bool {
        todo!("");
	}

	fn async_wait_root(&self) -> host::AsyncWaitRoot {
		struct AsyncWaitRoot {
            host: super::HostRef,
		}
		impl core::future::Future for AsyncWaitRoot {
			type Output = usize;
			fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
                use ::core::sync::atomic::Ordering;
                for (j,vp) in self.host.port_update.iter().enumerate()
                {
                    let v = vp.load(Ordering::SeqCst);
                    if v != 0
                    {
                        log_debug!("UsbHost::AsyncWaitRoot::poll: v[{}] = {:#x}", j, v);
                        for i in 0 .. 32
                        {
                            let bit = 1 << i;
                            if v & bit != 0 {
                                vp.fetch_and(!bit, Ordering::SeqCst);
                                return core::task::Poll::Ready(j * 32 + i);
                            }
                        }
                    }
                }
				*self.host.port_update_waker.lock() = cx.waker().clone();
				core::task::Poll::Pending
			}
		}
		usb_core::host::AsyncWaitRoot::new(AsyncWaitRoot {
			host: self.host.reborrow(),
			}).ok().expect("Over-size task in `async_wait_root`")
	}
}

/// Create an `AsyncWaitIo` instance (boxes if required)
fn make_asyncwaitio<'a, T>(f: impl ::core::future::Future<Output=T> + Send + Sync + 'a) -> host::AsyncWaitIo<'a, T> {
    host::AsyncWaitIo::new(f)
        .unwrap_or_else(|v| host::AsyncWaitIo::new(
            ::kernel::lib::mem::boxed::Box::pin(v)).ok().unwrap()
            )
}
