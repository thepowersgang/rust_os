// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/network_calls.rs
//! Userland interface to the network stack

pub fn new_free_socket(local_address: ::values::SocketAddress, remote_mask: ::values::MaskedSocketAddress) -> Result<u32, ::values::SocketError>
{
	if local_address.port_ty != remote_mask.addr.port_ty {
		return Err(::values::SocketError::InvalidValue);
	}
	if local_address.addr_ty != remote_mask.addr.addr_ty {
		return Err(::values::SocketError::InvalidValue);
	}
	// TODO: Check that the current process is allowed to use the specified combination of port/type
	todo!("new_free_socket");
}

struct FreeSocket
{
}

