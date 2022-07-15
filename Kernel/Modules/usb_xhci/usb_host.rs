
use ::usb_core::host;
use ::usb_core::host::{Handle,EndpointAddr};

pub struct UsbHost
{
    pub(crate) host: crate::ArefBorrow<crate::HostInner>,
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
        todo!("");
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
        todo!("");
	}
}
