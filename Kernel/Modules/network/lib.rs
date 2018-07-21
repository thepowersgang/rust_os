// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/lib.rs
//! Networking stack
#![no_std]
#![feature(linkage)]
#![feature(const_fn)] 
#![feature(no_more_cas)]	// AtomicUsize::fetch_update
#![feature(crate_in_paths)]

#[cfg(test)] #[macro_use] extern crate /**/ std;

#[macro_use]
extern crate kernel;
extern crate stack_dst;
extern crate shared_map;

module_define!{Network, [], init}

pub mod nic;
pub mod tcp;
pub mod arp;
pub mod ipv4;
//pub mod ipv6;

fn init()
{
}

