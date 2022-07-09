//
//
//
//!
use kernel::device_manager;
use crate::interface::Interface;

mod block;
mod video;
//mod network;
mod input;

pub fn new_boxed<T: Interface+Send+Sync+'static>(dev_id: u32, int: T) -> device_manager::DriverInstancePtr
{
	match dev_id
	{
	// 0: Reserved/invalid
	0 => device_manager::DriverInstancePtr::new( NullDevice ),
	//1 => Box::new( network::NetDevice::new(int) ),
	1 => {
		log_notice!("TODO: Support VirtIO network devices (type = 1)");
		device_manager::DriverInstancePtr::new(NullDevice)
		}
	2 => device_manager::DriverInstancePtr::new( block::BlockDevice::new(int) ),	// 2 = Block device
	// DISABLED: Changing video modes breaks stuff currently...
	16 => if true { 	// 16 = Graphics Adapter
			device_manager::DriverInstancePtr::new( video::VideoDevice::new(int) )
		}
		else {
			device_manager::DriverInstancePtr::new(NullDevice)
		},
	18 => device_manager::DriverInstancePtr::new(input::InputDevice::new(int)),
	dev @ _ => {
		log_error!("VirtIO device has unknown device ID {:#x}", dev);
		device_manager::DriverInstancePtr::new(NullDevice)
		},
	}
}

pub struct NullDevice;
impl device_manager::DriverInstance for NullDevice {
}
