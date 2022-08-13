/*
 */

//use handle::{Handle,RemoteFree};

pub use crate::hub::PortFeature;

/// A double-fat pointer (three words long)
pub type Handle<T/*: ?Sized*/> = ::stack_dst::ValueA<T, [usize; 3]>;

#[derive(Copy,Clone)]
pub struct EndpointAddr(u16);	// 7 bit device and 4 bit endpoint (encoded together)
impl EndpointAddr
{
	pub fn new(dev: u8, endpt: u8) -> EndpointAddr {
		assert!(dev < 128);
		assert!(endpt < 16);
		EndpointAddr(dev as u16 * 16 + endpt as u16)
	}
	pub fn dev_addr(&self) -> u8 {
		(self.0 >> 4) as u8
	}
	pub fn endpt(&self) -> u8 {
		(self.0 & 0xF) as u8
	}
}
impl ::core::fmt::Debug for EndpointAddr
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "{}:{}", self.dev_addr(), self.endpt())
	}
}

pub enum HubPortSpeed
{
	// USB 1.0 - 1.5Mbps
	Low,
	// USB 1.0 - 12Mbps
	Full,
	// USB 2.0 - 480Mbps
	High,

	// USB 3.0 - 5 Gb/s
	Super,
	SuperSpeedPlusG2,
	SuperSpeedPlusG1X2,
	SuperSpeedPlusG2X2,
}

pub type AsyncWaitIo<'a, T> = stack_dst::ValueA<dyn core::future::Future<Output=T> + Sync + Send + 'a, [usize; 3]>;
pub type IntBuffer<'a> = Handle<dyn crate::handle::RemoteBuffer + Send + Sync + 'a>;
//#[smart_ptr(::kernel::lib::mem::Box)]
pub trait InterruptEndpoint: Send + Sync
{
	fn wait<'a>(&'a self) -> AsyncWaitIo<'a, IntBuffer<'a>>;
}
impl<T: ?Sized + InterruptEndpoint> InterruptEndpoint for ::kernel::lib::mem::Box<T> {
	fn wait<'a>(&'a self) -> AsyncWaitIo<'a, IntBuffer<'a>> {
		(**self).wait()
	}
}

pub trait ControlEndpoint: Send + Sync
{
	// TODO: Have a type that abstracts that the data will be valid even if the future is leaked
	fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> AsyncWaitIo<'a, usize>;
	fn in_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a mut [u8]) -> AsyncWaitIo<'a, usize>;
}
impl<T: ?Sized + ControlEndpoint> ControlEndpoint for ::kernel::lib::mem::Box<T> {
	fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> AsyncWaitIo<'a, usize> {
		(**self).out_only(setup_data, out_data)
	}
	fn in_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a mut [u8]) -> AsyncWaitIo<'a, usize> {
		(**self).in_only(setup_data, out_data)
	}
}

pub trait IsochEndpoint: Send + Sync
{
	// /// Returns the current controller frame number (for timing) and the matching system time
	// fn get_current_frame_and_time(&self) -> (u32, ::kernel::time::TickCount);
	// /// Start a send to be sent at the specified frame (relative to controller's arbtiary basis)
	// fn send_at<'a, 's>(&'s self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, buffer: kasync::WriteBufferHandle<'a, '_>, abs_frame: u32);
	// /// Prepare a receive to complete in the specified frame.
	// fn recv_at<'a, 's>(&'s self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, buffer: &'a mut [u8], abs_frame: u32);
}
impl<T: ?Sized + IsochEndpoint> IsochEndpoint for ::kernel::lib::mem::Box<T> {
}

pub trait BulkEndpointOut: Send + Sync
{
	fn send<'a>(&'a self, buffer: &'a [u8]) -> AsyncWaitIo<'a, usize>;
}
impl<T: ?Sized + BulkEndpointOut> BulkEndpointOut for ::kernel::lib::mem::Box<T> {
	fn send<'a>(&'a self, buffer: &'a [u8]) -> AsyncWaitIo<'a, usize> {
		(**self).send(buffer)
	}
}

pub trait BulkEndpointIn: Send + Sync
{
	fn recv<'a>(&'a self, buffer: &'a mut [u8]) -> AsyncWaitIo<'a, usize>;
}
impl<T: ?Sized + BulkEndpointIn> BulkEndpointIn for ::kernel::lib::mem::Box<T> {
	fn recv<'a>(&'a self, buffer: &'a mut [u8]) -> AsyncWaitIo<'a, usize> {
		(**self).recv(buffer)
	}
}

pub type AsyncWaitRoot = stack_dst::ValueA<dyn core::future::Future<Output=usize>, [usize; 3]>;
pub trait HostController: Send + Sync
{
	// TODO: xHCI allocates the addresses itself

	///// Obtain a handle to endpoint zero
	//fn get_control_zero(&self) -> Handle<dyn ControlEndpoint>;
	/// Begin polling an endpoint at the given rate (buffer used is allocated by the driver to be the interrupt endpoint's size)
	fn init_interrupt(&self, endpoint: EndpointAddr, max_packet_size: usize, period_ms: usize) -> Handle<dyn InterruptEndpoint>;
	/// Initialise an ichronous endpoint
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn IsochEndpoint>;
	/// Initialise a control endpoint
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn ControlEndpoint>;
	/// Initialise a bulk endpoint for OUT
	fn init_bulk_out(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn BulkEndpointOut>;
	/// Initialise a bulk endpoint for IN
	fn init_bulk_in(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn BulkEndpointIn>;


	// Root hub maintainence
	//fn take_port_update_mask(&self) -> u32;
	fn set_port_feature(&self, port: usize, feature: PortFeature);
	fn clear_port_feature(&self, port: usize, feature: PortFeature);
	fn get_port_feature(&self, port: usize, feature: PortFeature) -> bool;

	/// Inform the driver of the speed of a port on a hub. This is called when the port becomes active (indicating the state of Dev0)
	/// 
	/// (This is hack for EHCI)
	fn set_hub_port_speed(&self, hub_endpoint_zero: &dyn ControlEndpoint, port: usize, speed: HubPortSpeed) {
		let _ = hub_endpoint_zero;
		let _ = port;
		let _ = speed;
	}

	fn async_wait_root(&self) -> AsyncWaitRoot;
}

