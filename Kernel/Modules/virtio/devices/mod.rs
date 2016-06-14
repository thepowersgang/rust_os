//
//
//
//!
use kernel::prelude::*;
use kernel::device_manager;
use interface::Interface;

mod block;
//mod network;

pub fn new_boxed<T: Interface+Send+'static>(dev: u32, io: device_manager::IOBinding, irq: u32) -> Box<device_manager::DriverInstance>
{
	match dev
	{
	// 0: Reserved/invalid
	0 => Box::new( NullDevice ),
	//1 => Box::new( network::NetDevice::new(T::new(io, irq)) ),
	1 => {
		log_notice!("TODO: Support VirtIO network devices (type = 1)");
		Box::new(NullDevice)
		}
	2 => Box::new( block::BlockDevice::new(T::new(io, irq)) ),
	dev @ _ => {
		log_error!("VirtIO device has unknown device ID {:#x}", dev);
		Box::new(NullDevice)
		},
	}
}

pub struct NullDevice;
impl device_manager::DriverInstance for NullDevice {
}
