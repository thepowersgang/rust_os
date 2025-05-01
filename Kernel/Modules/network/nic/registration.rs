//! Logic for creating a network card registration
use kernel::prelude::*;
use kernel::lib::mem::aref::Aref;
use core::sync::atomic::Ordering;
use super::INTERFACES_LIST;
use super::Interface;
use super::Error;

pub fn register<T: Interface>(mac_addr: [u8; 6], int: T) -> Registration<T> {
	let int_ptr = Aref::new(int);

	let int_data = kernel::lib::mem::Arc::new(super::InterfaceData {
		addr: mac_addr,
		stop_flag: Default::default(),
		sleep_object_ref: Default::default(),
		base_interface: int_ptr.borrow(),
		});
	let reg = super::InterfaceListEnt {
		data: int_data.clone(),
		thread: ::kernel::threads::WorkerThread::new("Network Rx", move || rx_thread(&int_data)),
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
	let index = insert_opt(&mut INTERFACES_LIST.lock(), reg);
	
	Registration {
		pd: ::core::marker::PhantomData,
		index,
		ptr: int_ptr,
		}
}

/// Handle to a registered interface
pub struct Registration<T> {
	// Logically owns the `T`
	pd: ::core::marker::PhantomData<T>,
	index: usize,
	ptr: Aref<T>,
}
impl<T> Drop for Registration<T> {
	fn drop(&mut self) {
		log_notice!("Dropping interface {:p}", &*self.ptr);
		let mut lh = INTERFACES_LIST.lock();
		assert!( self.index < lh.len() );
		if let Some(ref mut int_ent) = lh[self.index] {
			int_ent.data.stop_flag.store(true, Ordering::SeqCst);
			int_ent.data.sleep_object_ref.lock().take().unwrap().signal();
			int_ent.thread.wait().expect("Couldn't wait for NIC worker to terminate");
			// TODO: Inform the rest of the stack that this interface is gone?
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

fn rx_thread(interface: &super::InterfaceData)
{
	::kernel::threads::SleepObject::with_new("rx_thread", |so| {
		*interface.sleep_object_ref.lock() = Some(so.get_ref());
		interface.base_interface.rx_wait_register(&so);
		while !interface.stop_flag.load(Ordering::SeqCst)
		{
			so.wait();
			match interface.base_interface.rx_packet()
			{
			Ok(pkt) => {
				log_notice!("Received packet, len={} (chunks={})", pkt.len(), pkt.num_regions());
				for r in 0 .. pkt.num_regions() {
					log_debug!("{} {:?}", r, ::kernel::logging::HexDump(pkt.get_region(r)));
				}
				// TODO: Should this go in is own module?
				// 1. Interpret the `Ethernet II` header
				if pkt.len() < 6+6+2 {
					log_notice!("Short packet ({} < {})", pkt.len(), 6+6+2);
					continue ;
				}
				let mut reader = super::PacketReader::new(&pkt);
				// 2. Hand off to sub-modules depending on the EtherTy field
				let _dest_mac = {
					let mut b = [0; 6];
					reader.read(&mut b).unwrap();
					b
					};
				let source_mac = {
					let mut b = [0; 6];
					reader.read(&mut b).unwrap();
					b
					};
				let ether_ty = reader.read_u16n().unwrap();
				match ether_ty
				{
				// IPv4
				0x0800 => match crate::ipv4::handle_rx_ethernet(&interface, source_mac, reader)
					{
					Ok( () ) => {},
					Err(e) => {
						log_warning!("TODO: Unable to handle IPv4 packet - {:?}", e);
						},
					},
				// ARP
				0x0806 => {
					crate::arp::handle_packet(&*interface.base_interface, source_mac, reader);
					},
				// IPv6
				0x86DD => match crate::ipv6::handle_rx_ethernet(&interface, source_mac, reader)
					{
					Ok(()) => {},
					Err(e) => log_warning!("TODO: Unable to handle IPv6 packet - {:?}", e),
					},
				v @ _ => {
					log_warning!("TODO: Handle packet with EtherTy={:#x}", v);
					},
				}
				},
			Err(Error::NoPacket) => {},
			Err(e) => todo!("{:?}", e),
			}
		}
		log_debug!("Worker termination requested");
		interface.base_interface.rx_wait_unregister(&so);
		// NOTE: Lock the reference slot, so the reference is deleted before the sleep object quits
		let lh = interface.sleep_object_ref.lock();
		assert!(lh.is_none());
		});
}

