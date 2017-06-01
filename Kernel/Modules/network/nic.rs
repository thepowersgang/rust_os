// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/nic.rs
//! "Network Interface Card" interface
use kernel::prelude::*;
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use kernel::sync::Mutex;

#[derive(Debug)]
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

pub type PacketHandle<'a> = ::stack_dst::ValueA<RxPacket + 'a, [usize; 8]>;
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
	fn rx_wait_register(&self, channel: &::kernel::threads::SleepObject);
	
	/// Obtain a packet from the interface (or `Err(Error::NoPacket)` if there is none)
	fn rx_packet(&self) -> Result<PacketHandle, Error>;
}

struct InterfaceData
{
	#[allow(dead_code)]	// Never read, just exists to hold the handle
	base_interface: Aref<Interface+'static>,
	thread: ::kernel::threads::WorkerThread,
}

static INTERFACES_LIST: Mutex<Vec< Option<InterfaceData> >> = Mutex::new(Vec::new_const());

/// Handle to a registered interface
pub struct Registration<T> {
	// Logically owns the `T`
	pd: ::core::marker::PhantomData<T>,
	index: usize,
	ptr: ArefBorrow<T>,
}
impl<T> Drop for Registration<T> {
	fn drop(&mut self) {
		let mut lh = INTERFACES_LIST.lock();
		assert!( self.index < lh.len() );
		if let Some(ref int_ent) = lh[self.index] {
			//int_ent.stop_signal.set();
			int_ent.thread.wait().expect("Couldn't wait for NIC worker to terminate");
		}
		else {
			panic!("NIC registration pointed to unpopulated entry");
		}
		lh[self.index] = None;
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

	// HACK: Send a dummy packet
	// - An ICMP Echo request to qemu's user network router
	{
		let mut pkt = 
			// MAC Dst                MAC Src     EtherTy IP      TotalLen Identif Frag   TTL Prot CkSum  Source          Dest            ICMP
			*b"\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\x08\x00\x45\x00\x00\x23\x00\x00\x00\x00\xFF\x01\xa3\xca\x0A\x00\x02\x0F\x0A\x00\x02\x01\x08\x00\x7d\x0d\x00\x00\x00\x00Hello World"
			;
		pkt[6..][..6].copy_from_slice( &mac_addr );
		reg.tx_raw(SparsePacket { head: &pkt, next: None });
	}

	let worker_reg = reg.borrow();
	let reg = InterfaceData {
		thread: ::kernel::threads::WorkerThread::new("Network Rx", move || rx_thread(&*worker_reg)),
		base_interface: reg,
		};

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
	let idx = insert_opt(&mut INTERFACES_LIST.lock(), reg);
	
	Registration {
		pd: ::core::marker::PhantomData,
		index: idx,
		ptr: b,
		}
}

fn rx_thread(int: &Interface)
{
	loop
	{
		let so = ::kernel::threads::SleepObject::new("rx_thread");
		int.rx_wait_register(&so);
		so.wait();
		match int.rx_packet()
		{
		Ok(pkt) => todo!("Received packet - len={}", pkt.len()),
		Err(Error::NoPacket) => {},
		Err(e) => todo!("{:?}", e),
		}
	}
}

