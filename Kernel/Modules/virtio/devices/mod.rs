//
//
//
//!
use kernel::prelude::*;
use kernel::device_manager;
use interface::Interface;

mod block;
mod video;
//mod network;

pub fn new_boxed<T: Interface+Send+Sync+'static>(dev_id: u32, int: T) -> Box<device_manager::DriverInstance>
{
	match dev_id
	{
	// 0: Reserved/invalid
	0 => Box::new( NullDevice ),
	//1 => Box::new( network::NetDevice::new(int) ),
	1 => {
		log_notice!("TODO: Support VirtIO network devices (type = 1)");
		Box::new(NullDevice)
		}
	2 => Box::new( block::BlockDevice::new(int) ),	// 2 = Block device
	16 => Box::new( video::VideoDevice::new(int) ),	// 16 = Graphics Adapter
	dev @ _ => {
		log_error!("VirtIO device has unknown device ID {:#x}", dev);
		Box::new(NullDevice)
		},
	}
}

pub struct NullDevice;
impl device_manager::DriverInstance for NullDevice {
}
