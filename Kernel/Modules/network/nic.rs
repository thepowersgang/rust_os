// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/nic.rs
//! "Network Interface Card" interface
use kernel::prelude::*;
use kernel::sync::Mutex;
//use kernel::_async3 as async;
use core::sync::atomic::AtomicBool;

mod packet;
pub use self::packet::{RxPacket, PacketHandle};
pub use self::packet::{SparsePacket, SparsePacketIter};
pub use self::packet::PacketReader;

mod registration;
pub use self::registration::{register, Registration};

pub type MacAddr = [u8; 6];

#[derive(Debug)]
pub enum Error
{
	/// No packets waiting
	NoPacket,
	/// An oversized packet was received
	MtuExceeded,
	/// Not enough space available for the packet
	BufferUnderrun,
	///// Async stack space exceeded
	//AsyncTooDeep,
}

/// Network interface API
pub trait Interface: 'static + Send + Sync
{
	/// Transmit a raw packet (blocking)
	fn tx_raw(&self, pkt: SparsePacket);

	/// The input buffer can be a mix of `> 'stack` and `< 'stack` buffers. This function should collapse shorter lifetime
	/// buffers into an internal buffer that lives long enough.
	//fn tx_async<'a, 's>(&'s self, async: async::ObjectHandle, stack: async::StackPush<'a, 's>, pkt: SparsePacket) -> Result<(), Error>;

	/// Called once to allow the interface to get an object to signal a new packet arrival
	fn rx_wait_register(&self, channel: &::kernel::threads::SleepObject);
	fn rx_wait_unregister(&self, channel: &::kernel::threads::SleepObject);
	
	/// Obtain a packet from the interface (or `Err(Error::NoPacket)` if there is none)
	/// - Non-blocking
	fn rx_packet(&self) -> Result<PacketHandle, Error>;
}

struct InterfaceData
{
	addr: MacAddr,
	stop_flag: AtomicBool,
	base_interface: ::kernel::lib::mem::aref::ArefBorrow<dyn Interface+'static>,

	sleep_object_ref: Mutex<Option<kernel::threads::SleepObjectRef>>,
}
struct InterfaceListEnt
{
	data: ::kernel::lib::mem::Arc<InterfaceData>,
	thread: ::kernel::threads::WorkerThread,
}

static INTERFACES_LIST: Mutex<Vec< Option<InterfaceListEnt> >> = Mutex::new(Vec::new());

/// Send a packaet from the interface matching the interface matching `local_addr`
pub fn send_from(local_addr: MacAddr, dest_addr: MacAddr, ether_ty: u16, pkt: SparsePacket)
{
	let mut int = None;
	for i in INTERFACES_LIST.lock().iter()
	{
		if let Some(v) = i
		{
			if v.data.addr == local_addr
			{
				int = Some(v.data.clone());
			}
		}
	}
	if let Some(i) = int
	{
		// Create the ethernet header
		let buf = [
			dest_addr[0], dest_addr[1], dest_addr[2], dest_addr[3], dest_addr[4], dest_addr[5],
			local_addr[0], local_addr[1], local_addr[2], local_addr[3], local_addr[4], local_addr[5],
			(ether_ty >> 8) as u8, ether_ty as u8,
			];
		i.base_interface.tx_raw(SparsePacket::new_chained(&buf, &pkt));
	}
}

/// Returns the number of allocated interface slots (some of which may be unused)
pub fn count_interfaces() -> usize
{
	INTERFACES_LIST.lock().len()
}
/// User-visible information about a network interface (e.g. card)
/// 
/// Return type from [interface_info]
pub struct InterfaceInfo {
	/// Physical layer (MAC) address
	pub mac: MacAddr,
}
/// Get information about a possible network interface
/// 
/// See also [count_interfaces]
pub fn interface_info(index: usize) -> Option<InterfaceInfo> {
	match INTERFACES_LIST.lock().get(index)
	{
	Some(Some(v)) => Some(InterfaceInfo {
		mac: v.data.addr,
	}),
	_ => None,
	}
}