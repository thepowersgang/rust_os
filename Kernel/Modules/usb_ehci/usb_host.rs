use ::core::sync::atomic::Ordering;
use ::usb_core::host::{self,PortFeature,EndpointAddr,Handle};

pub struct UsbHost
{
    pub(crate) host: super::HostRef,
}
impl ::usb_core::host::HostController for UsbHost
{
	fn init_interrupt(&self, endpoint: EndpointAddr, period_ms: usize, max_packet_size: usize) -> Handle<dyn host::InterruptEndpoint> {
        todo!("init_interrupt")
	}
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::IsochEndpoint> {
		todo!("init_isoch({:?}, max_packet_size={})", endpoint, max_packet_size);
	}
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::ControlEndpoint> {
        todo!("init_control")
	}
	fn init_bulk_out(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointOut> {
        todo!("init_bulk_out")
	}
	fn init_bulk_in(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointIn> {
        todo!("init_bulk_in")
	}


	// Root hub maintainence
	fn set_port_feature(&self, port: usize, feature: PortFeature) {
        if let Some(bit) = feature_bit(feature, FeatureOp::Set)  {
            let v = self.host.regs.read_port_sc(port as u8);
            unsafe { self.host.regs.write_port_sc(port as u8, v | bit); }
        }
        else {
        }
	}
	fn clear_port_feature(&self, port: usize, feature: PortFeature) {
        if let Some(bit) = feature_bit(feature, FeatureOp::Clear)  {
            let v = self.host.regs.read_port_sc(port as u8);
            unsafe { self.host.regs.write_port_sc(port as u8, v & !bit); }
        }
        else {
        }
	}
	fn get_port_feature(&self, port: usize, feature: PortFeature) -> bool {
        if let Some(bit) = feature_bit(feature, FeatureOp::Get)  {
            self.host.regs.read_port_sc(port as u8) & bit != 0
        }
        else {
            false
        }
	}

	fn async_wait_root(&self) -> host::AsyncWaitRoot {
		struct AsyncWaitRoot {
            host: super::HostRef,
		}
		impl core::future::Future for AsyncWaitRoot {
			type Output = usize;
			fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
				let v = self.host.port_update.load(Ordering::SeqCst);
				log_debug!("UsbHost::AsyncWaitRoot::poll: v = {:#x}", v);
				if v != 0
				{
					for i in 0 .. self.host.nports()
					{
						let bit = 1 << i;
						if v & bit != 0 {
							self.host.port_update.fetch_and(!bit, Ordering::SeqCst);
							return core::task::Poll::Ready(i as usize);
						}
					}
				}
				*self.host.waker.lock() = cx.waker().clone();
				core::task::Poll::Pending
			}
		}
		usb_core::host::AsyncWaitRoot::new(AsyncWaitRoot {
			host: self.host.reborrow(),
			}).ok().expect("Over-size task in")
	}
}
#[derive(PartialOrd,PartialEq)]
enum FeatureOp {
    /// All features support get
    Get,
    /// Some can only be cleared
    Clear,
    /// And even fewer can be set
    Set,
}
fn feature_bit(feature: PortFeature, o: FeatureOp) -> Option<u32> {
    let only_get = |bit| if o > FeatureOp::Get { None } else { Some(bit) };
    let no_set = |bit| if o > FeatureOp::Clear { None } else { Some(bit) };
    Some(match feature 
    {
    PortFeature::Connection  => only_get(crate::hw_regs::PORTSC_CurrentConnectStatus)?,
    PortFeature::Enable      => crate::hw_regs::PORTSC_PortEnabled,
    PortFeature::Suspend     => crate::hw_regs::PORTSC_Suspend,
    PortFeature::OverCurrent => only_get(crate::hw_regs::PORTSC_OvercurrentActive)?,
    PortFeature::Reset       => crate::hw_regs::PORTSC_PortReset,
    PortFeature::Power       => crate::hw_regs::PORTSC_PortPower,
    PortFeature::LowSpeed    => return None,
    PortFeature::CConnection => no_set(crate::hw_regs::PORTSC_ConnectStatusChange)?,
    PortFeature::CEnable     => no_set(crate::hw_regs::PORTSC_PortEnableChange)?,
    PortFeature::CSuspend    => return None,
    PortFeature::COverCurrent=> no_set(crate::hw_regs::PORTSC_OvercurrentChange)?,
    PortFeature::CReset      => return None,
    PortFeature::Test        => return None,
    PortFeature::Indicator   => match o
        {
        FeatureOp::Get   => crate::hw_regs::PORTSC_PortIndicator_MASK,
        FeatureOp::Clear => crate::hw_regs::PORTSC_PortIndicator_MASK,
        FeatureOp::Set   => crate::hw_regs::PORTSC_PortIndicator_Green,
        },
    })
}
