#![feature(no_std,linkage)]
#![no_std]
#![feature(core_slice_ext)]
#![feature(associated_consts)]

#[macro_use] extern crate kernel;

module_define!{VirtIO, [DeviceManager, Storage], init}

mod drivers;

fn init()
{
	drivers::register();
}

