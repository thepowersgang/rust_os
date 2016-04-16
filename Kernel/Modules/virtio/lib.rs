// "Tifflin" Kernel - VirtIO Driver
// - By John Hodge (thePowersGang)
//
// virtio/lib.rs
//! Virtual IO devices
#![no_std]
#![feature(linkage)]

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

