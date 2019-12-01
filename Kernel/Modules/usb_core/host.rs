/*
 */
use kernel::_async3 as kasync;

//use handle::{Handle,RemoteFree};

pub use crate::hub::PortFeature;

/// A double-fat pointer (three words long)
pub type Handle<T/*: ?Sized*/> = ::stack_dst::ValueA<T, [usize; 3]>;

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

pub trait InterruptEndpoint: Send + Sync
{
	fn get_data(&self) -> Handle<dyn crate::handle::RemoteBuffer>;
}
//	fn tx_async<'a, 's>(&'s self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, pkt: SparsePacket) -> Result<(), Error>;
pub trait ControlEndpoint
{
	//fn out_only<'a, 'b>(&self, async_pool: &'a kasync::Pool, setup_data: kasync::WriteBuffer<'b>) -> kasync::Alloc<'a, dyn Future<Output=usize> + 'b>;
	fn out_only<'a, 's>(&'s self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, setup_data: kasync::WriteBufferHandle<'s, '_>, out_data: kasync::WriteBufferHandle<'s, '_>);
	fn in_only<'a, 's>(&'s self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, setup_data: kasync::WriteBufferHandle<'s, '_>, in_buf: &'s mut [u8]);
	// The following are more interesting, `out/in` works, but `in/out` has ordering problems...
	// - Thankfully, these patterns aren't needed?
	//fn out_in(&self, waiter: kasync::WaiterHandle, out_data: kasync::WriteBufferHandle, in_buf: kasync::ReadBufferHandle);
	//fn in_out(&self, waiter: kasync::WaiterHandle, in_buf: kasync::ReadBufferHandle, out_data: kasync::WriteBufferHandle);
}
pub trait IsochEndpoint: Send + Sync
{
	/// Returns the current controller frame number (for timing) and the matching system time
	fn get_current_frame_and_time(&self) -> (u32, ::kernel::time::TickCount);
	/// Start a send to be sent at the specified frame (relative to controller's arbtiary basis)
	fn send_at<'a, 's>(&'s self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, buffer: kasync::WriteBufferHandle<'a, '_>, abs_frame: u32);
	/// Prepare a receive to complete in the specified frame.
	fn recv_at<'a, 's>(&'s self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, buffer: &'a mut [u8], abs_frame: u32);
}
pub trait BulkEndpoint: Send + Sync
{
	// Start a send operation of the passed buffers
	fn send<'a, 's>(&self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, buffer: kasync::WriteBufferHandle<'a, '_>);
	fn recv<'a, 's>(&self, async_obj: kasync::ObjectHandle, stack: kasync::StackPush<'a, 's>, buffer: &'a mut [u8]);
}

pub trait HostController: Send + Sync
{
	///// Obtain a handle to endpoint zero
	//fn get_control_zero(&self) -> Handle<dyn ControlEndpoint>;
	/// Begin polling an endpoint at the given rate (buffer used is allocated by the driver to be the interrupt endpoint's size)
	fn init_interrupt(&self, endpoint: EndpointAddr, period_ms: usize, waiter: kasync::ObjectHandle) -> Handle<dyn InterruptEndpoint>;
	/// Initialise an ichronous endpoint
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn IsochEndpoint>;
	/// Initialise a control endpoint
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn ControlEndpoint>;
	/// Initialise a bulk endpoint
	fn init_bulk(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn BulkEndpoint>;


	// Root hub maintainence
	//fn take_port_update_mask(&self) -> u32;
	fn set_port_feature(&self, port: usize, feature: PortFeature);
	fn clear_port_feature(&self, port: usize, feature: PortFeature);
	fn get_port_feature(&self, port: usize, feature: PortFeature) -> bool;

	/// Register a queue of (my_idx,port_num) pairs for changes to the root hub
	fn set_root_waiter(&mut self, waiter: &'static ::kernel::sync::Queue<(usize,usize)>, my_idx: usize);
}

