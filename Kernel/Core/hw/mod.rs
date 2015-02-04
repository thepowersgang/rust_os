// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/mod.rs
// - Core hardware drivers

pub mod bus_pci;

/// Boot-time video support (using a bootloader-provided buffer)
// NOTE: Is public so the bootloader can use the types defined within
pub mod bootvideo;

// vim: ft=rust

