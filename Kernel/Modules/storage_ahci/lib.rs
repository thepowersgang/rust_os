// "Tifflin" Kernel - AHCI (SATA) Driver
// - By John Hodge (thePowersGang)
//
// Modules/storage_ahci/lib.rs
//! AHCI Driver
#![feature(linkage)]
#![no_std]

#[macro_use]
extern crate kernel;

extern crate storage_ata;
extern crate storage_scsi;

module_define!{AHCI, [DeviceManager, Storage], init}

mod bus_bindings;
mod hw;

mod controller;
mod port;

fn init()
{
	::kernel::device_manager::register_driver(&bus_bindings::S_PCI_DRIVER);
}


