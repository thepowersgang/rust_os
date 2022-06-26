//! Implementation of the `usb_core` `HostController` trait
use ::core::sync::atomic::Ordering;
use ::kernel::prelude::Box;
use ::usb_core::host::{self,PortFeature,EndpointAddr,Handle};

mod control_endpoint;
mod bulk_endpoint;
mod interrupt_endpoint;
use self::control_endpoint::ControlEndpoint;
use self::bulk_endpoint::BulkEndpoint;
use self::interrupt_endpoint::InterruptEndpoint;

pub struct UsbHost
{
    pub(crate) host: super::HostRef,
}
impl ::usb_core::host::HostController for UsbHost
{
	fn init_interrupt(&self, endpoint: EndpointAddr, period_ms: usize, max_packet_size: usize) -> Handle<dyn host::InterruptEndpoint> {
        Handle::new( Box::new(
            InterruptEndpoint::new(self.host.clone(), endpoint, period_ms, max_packet_size)
        )).ok().expect("Cannot fit Box in Handle")
	}
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::IsochEndpoint> {
		todo!("init_isoch({:?}, max_packet_size={})", endpoint, max_packet_size);
	}
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::ControlEndpoint> {
        Handle::new( Box::new(
            ControlEndpoint::new(self.host.clone(), endpoint, max_packet_size)
        )).ok().expect("Cannot fit Box in Handle")
	}
	fn init_bulk_out(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointOut> {
        Handle::new( Box::new(
            BulkEndpoint::new(self.host.clone(), endpoint, max_packet_size)
        )).ok().expect("Cannot fit Box in Handle")
	}
	fn init_bulk_in(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointIn> {
        Handle::new( Box::new(
            BulkEndpoint::new(self.host.clone(), endpoint, max_packet_size)
        )).ok().expect("Cannot fit Box in Handle")
	}


	// Root hub maintainence
	fn set_port_feature(&self, port: usize, feature: PortFeature) {
        if let Some(bit) = feature_bit(feature, FeatureOp::Set)  {
            log_debug!("set_port_feature({port} {feature:?}): {bit:#x}");
            let v = self.host.regs.read_port_sc(port as u8);
            // SAFE: Correct bits written
            unsafe { self.host.regs.write_port_sc(port as u8, v | bit); }
        }
        else {
        }
	}
	fn clear_port_feature(&self, port: usize, feature: PortFeature) {
        if let Some(bit) = feature_bit(feature, FeatureOp::Clear)  {
            log_debug!("clear_port_feature({port} {feature:?}): {bit:#x}");
            let v = self.host.regs.read_port_sc(port as u8);
            // SAFE: Correct bits written
            unsafe { self.host.regs.write_port_sc(port as u8, v & !bit); }
        }
        else {
        }
	}
	fn get_port_feature(&self, port: usize, feature: PortFeature) -> bool {
        if let Some(bit) = feature_bit(feature, FeatureOp::Get)  {
            let rv = self.host.regs.read_port_sc(port as u8) & bit != 0;
            log_debug!("get_port_feature({port} {feature:?}): {bit:#x} = {}", rv);
            rv
        }
        else {
            false
        }
	}

    /// This is called when a port actives, and informs the driver of Dev0's speed
    /// - That will be propagated later on
    fn set_hub_port_speed(&self, hub_endpoint_zero: &dyn host::ControlEndpoint, port: usize, speed: host::HubPortSpeed)
    {
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
        let hub_addr = hub_endpoint_zero.get_dev_addr();
        // This is only called when a device is entering setup
        // - Record that Dev0 needs to update to this info
        self.host.set_usb1(0, match speed
            {
            // Low/Full is USB1
            host::HubPortSpeed::Low|host::HubPortSpeed::Full => Some(Usb1 {
                // Speed is always direct
                is_fullspeed: matches!(speed, host::HubPortSpeed::Full),
                // Hub info is inherited from the hub (if it was a USB1 device)
                ..match self.host.get_usb1(hub_addr)
                    {
                    None => Usb1 { hub_addr, hub_port: port as u8, is_fullspeed: false, },
                    Some(p) => p,
                    }
                }),
            // High speed = USB2
            host::HubPortSpeed::High => None,
            });
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


/// Information about USB1 handling of a device
#[derive(Copy,Clone)]
pub struct Usb1 {
    hub_addr: u8,
    hub_port: u8,
    is_fullspeed: bool,
}

/// Create values for the `endpoint` and `endpoint_ext` fields of a queue head
fn make_endpoint_spec(endpoint: EndpointAddr, max_packet_size: usize, usb1: Option<Usb1>, is_control: bool) -> (u32, u32)
{
    let mut endpoint_id = 0
        | (max_packet_size as u32) << 16
        | (endpoint.dev_addr() as u32)
        | (endpoint.endpt() as u32) << 8;
    let mut endpoint_ext = 0
        | (0b01 << 30)  // Bandwidth multipler
        // Low 16 bits not used for async (control/bulk) endpoints
        ;
    set_usb1_state(&mut endpoint_id, &mut endpoint_ext, usb1, is_control);
    (endpoint_id, endpoint_ext)
}
/// Update endpoint description for a TD with new `Usb1` state
fn set_usb1_state(endpoint_id: &mut u32, endpoint_ext: &mut u32, usb1: Option<Usb1>, is_control: bool) {
    *endpoint_id = (*endpoint_id & !0x0800_3000)
        | if is_control && usb1.is_some() { 1 << 27 } else { 0 }  // Control flag
        | match usb1 {
            Some(Usb1 { is_fullspeed: true, .. }) => 0b00,   // Full speed
            Some(Usb1 { is_fullspeed: false, .. }) => 0b01,  // Low speed
            None => 0b10,   // High speed (USB2)
            } << 12
        ;
    *endpoint_ext = (*endpoint_ext & !0x3FFF_0000)
        | if let Some(Usb1 { hub_port, .. }) = usb1 { (hub_port as u32) << 23 } else { 0 }
        | if let Some(Usb1 { hub_addr, .. }) = usb1 { (hub_addr as u32) << 16 } else { 0 }
        ;
}
/// Create an `AsyncWaitIo` instance (boxes if required)
fn make_asyncwaitio<'a, T>(f: impl ::core::future::Future<Output=T> + Send + Sync + 'a) -> host::AsyncWaitIo<'a, T> {
    host::AsyncWaitIo::new(f)
        .unwrap_or_else(|v| host::AsyncWaitIo::new(
            ::kernel::lib::mem::boxed::Box::pin(v)).ok().unwrap()
            )
}

