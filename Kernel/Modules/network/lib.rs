// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/lib.rs
//! Networking stack
#![no_std]
#![feature(linkage)]
#![feature(const_fn)]

#[macro_use]
extern crate kernel;
extern crate stack_dst;

module_define!{Network, [], init}

pub mod nic;
pub mod tcp;
pub mod arp;
pub mod ipv4;
//pub mod ipv6;

fn init()
{
}

