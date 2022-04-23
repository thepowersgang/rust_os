// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/lib.rs
//! Networking stack
#![no_std]
#![feature(linkage)]

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
	crate::tcp::init();
}

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq,Debug)]
pub enum Address
{
	Ipv4(crate::ipv4::Address),
}
impl Address
{
	fn unwrap_ipv4(&self) -> crate::ipv4::Address {
		match self {
		&Address::Ipv4(v) => v,
		}
	}
}

