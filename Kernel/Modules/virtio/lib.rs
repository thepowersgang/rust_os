#![feature(no_std,linkage)]
#![no_std]
#![feature(core_slice_ext)]
#![feature(raw)]	// Used for unsized struct construction

#[macro_use] extern crate kernel;

module_define!{VirtIO, [DeviceManager, Storage], init}

mod drivers;
mod interface;
mod devices;
mod queue;

fn init()
{
	drivers::register();
}

