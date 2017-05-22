// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/traits.rs
//! Inter-layer traits

pub trait Transport
{
	pub fn send_packet(data: &[u8])
}

