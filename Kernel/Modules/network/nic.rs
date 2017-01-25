// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/nic.rs
//! "Network Interface Card" interface
use kernel::prelude::*;
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use kernel::sync::Mutex;

pub enum Error
{
	/// No packets waiting
	NoPacket,
	/// An oversized packet was received
	MtuExceeded,
	/// Not enough space avaliable for the packet
	BufferUnderrun,
}

/// Chain of wrapping packet information
pub struct SparsePacket<'a>
{
	head: &'a [u8],
	next: Option<&'a SparsePacket<'a>>,
}
impl<'a> IntoIterator for &'a SparsePacket<'a>
{
	type IntoIter = SparsePacketIter<'a>;
	type Item = &'a [u8];
	fn into_iter(self) -> SparsePacketIter<'a> {
		SparsePacketIter(Some(self))
	}
}
pub struct SparsePacketIter<'a>(Option<&'a SparsePacket<'a>>);
impl<'a> Iterator for SparsePacketIter<'a> {
	type Item = &'a [u8];
	fn next(&mut self) -> Option<Self::Item> {
		let p = match self.0
			{
			None => return None,
			Some(p) => p,
			};

		self.0 = p.next;
		Some(p.head)
	}
}

pub type PacketHandle<'a> = ::stack_dst::StackDST<RxPacket + 'a>;
pub trait RxPacket
{
	fn len(&self) -> usize;
	fn num_regions(&self) -> usize;
	fn get_region(&self, idx: usize) -> &[u8];
	fn get_slice(&self, range: ::core::ops::Range<usize>) -> Option<&[u8]>;
}

/// Network interface API
pub trait Interface: 'static + Send + Sync
{
	/// Transmit a raw packet
	fn tx_raw(&self, pkt: SparsePacket);

	// TODO: This interface is wrong, Waiter is the trait that bounds waitable objects (Use SleepObject instead)
	/// Called once to allow the interface to get an object to signal a new packet arrival
	fn rx_wait_register(&self, channel: &::kernel::async::Waiter);
	
	/// Obtain a packet from the interface (or `Err(Error::NoPacket)` if there is none)
	fn rx_packet(&self) -> Result<PacketHandle, Error>;
}

static INTERFACES_LIST: Mutex<Vec< Option<Aref<Interface>> >> = Mutex::new(Vec::new_const());

/// Handle to a registered interface
pub struct Registration<T> {
	pd: ::core::marker::PhantomData<T>,
	index: usize,
	ptr: ArefBorrow<T>,
}
impl<T> Drop for Registration<T> {
	fn drop(&mut self) {
		// TODO: Poke registration and tell it to remove
	}
}
impl<T> ::core::ops::Deref for Registration<T> {
	type Target = T;
	fn deref(&self) -> &T {
		&*self.ptr
	}
}

pub fn register<T: Interface>(mac_addr: [u8; 6], int: T) -> Registration<T> {
	let reg = Aref::new(int);
	let b = reg.borrow();

	fn insert_opt<T>(list: &mut Vec<Option<T>>, val: T) -> usize {
		for (i,s) in list.iter_mut().enumerate() {
			if s.is_none() {
				*s = Some(val);
				return i;
			}
		}
		list.push( Some(val) );
		return list.len() - 1;
	}

	// HACK: Send a dummy packet
	reg.tx_raw(SparsePacket { head: b"Hello World", next: None });

	let idx = insert_opt(&mut INTERFACES_LIST.lock(), reg);
	
	Registration {
		pd: ::core::marker::PhantomData,
		index: idx,
		ptr: b,
		}
}

