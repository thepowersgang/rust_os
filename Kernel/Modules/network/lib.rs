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
pub mod udp;

pub mod arp;
pub mod ipv4;
pub mod ipv6;

fn init()
{
	crate::tcp::init();
	crate::udp::init();
}

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq,Debug)]
pub enum Address
{
	Ipv4(crate::ipv4::Address),
	Ipv6(crate::ipv6::Address),
}
impl ::core::fmt::Display for Address {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
		Address::Ipv4(a) => a.fmt(f),
		Address::Ipv6(a) => a.fmt(f),
		}
	}
}
impl Address
{
	#[track_caller]
	fn unwrap_ipv4(&self) -> crate::ipv4::Address {
		match self {
		&Address::Ipv4(v) => v,
		_ => panic!(),
		}
	}
	#[track_caller]
	fn unwrap_ipv6(&self) -> crate::ipv6::Address {
		match self {
		&Address::Ipv6(v) => v,
		_ => panic!(),
		}
	}
	fn mask_network(&self, bits: u8) -> Self {
		match self {
		Address::Ipv4(address) => Address::Ipv4(address.mask_net(bits)),
		Address::Ipv6(address) => Address::Ipv6(address.mask_net(bits)),
		}
	}
}

