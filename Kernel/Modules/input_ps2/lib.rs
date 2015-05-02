// "Tifflin" Kernel - ATA Driver
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/lib.rs
//! PS2 Keyboard/Mouse controller
#![feature(no_std,core)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::_common::*;

// HACK: Requires USB to be active to ensure that emulation is off
module_define!{PS2, [DeviceManager, ACPI, GUI/*, USB*/], init}

#[derive(Default)]
struct PS2Dev;

mod i8042;

fn init()
{
	i8042::init();
}
