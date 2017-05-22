// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/lib.rs
//! Networking stack
#![no_std]
#![feature(linkage)]
#![feature(const_fn)]
#![feature(drop_types_in_const)]

#[macro_use]
extern crate kernel;
extern crate stack_dst;

module_define!{Network, [], init}

pub mod nic;
pub mod tcp;

fn init()
{
}

