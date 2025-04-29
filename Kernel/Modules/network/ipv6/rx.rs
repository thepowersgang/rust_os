use kernel::lib::Vec;
use kernel::sync::RwLock;
use crate::nic::PacketReader;
use crate::nic::MacAddr;
use super::{Address,Ipv6Header};
use super::{Interface,INTERFACES};

static PROTOCOL_HANDLDERS: RwLock<Vec<(u8, ProtoHandler)>> = RwLock::new(Vec::new());

pub fn handle_rx_ethernet(_physical_interface: &dyn crate::nic::Interface, iface_mac: MacAddr, source_mac: MacAddr, mut reader: PacketReader) -> Result<(), ()>
{
	let hdr = match Ipv6Header::read(&mut reader)
		{
		Ok(v) => v,
		Err(_) => {
			log_warning!("Undersized packet: Ran out of data reading header");
			return Err( () );
			},
		};
	if hdr.ver_tc_fl >> 28 != 6 {
		log_warning!("Malformed packet: version isn't 6 - ver_tc_fl={:08x}", hdr.ver_tc_fl);
		return Err( () );
	}

	// Apply routing (look up a matching interface)

	// Check extension headers
	let mut next_header = hdr.next_header;
	loop {
		next_header = match hdr.next_header {
			0 => todo!("Hop-by-hop options"),
			43 => todo!("Routing Header"),
			44 => todo!("Fragment Header"),
			60 => {
				let next = reader.read_u8()?;
				let len = reader.read_u8()? as usize * 8;
				for _ in 0 .. len {
					reader.read_u8()?;
				}
				next
			},
			59 => return Ok(()),
			_ => break,
			};
	}

	// Check destination IP against known interfaces.
	// - Could also be doing routing.
	for interface in INTERFACES.read().iter()
	{
		// TODO: Interfaces should be locked to the physical interface too
		if interface.local_mac == iface_mac && (interface.address == hdr.destination || hdr.destination == Address::broadcast())
		{
			if hdr.source.mask_net(interface.mask) == interface.address.mask_net(interface.mask) {
				// Snoop the source MAC into the neighbour-discovery cache
				super::nd::learn(iface_mac, source_mac, hdr.source, super::nd::LearnSource::Snoop);
			}

			// TODO: ICMPv6 handling
			// - Needs to include pings and status replies
			if next_header == super::icmpv6::NEXT_HEADER {
				// ICMPv6 (includes ND, type 133)
				super::icmpv6::handle_packet(interface, hdr.source, hdr.destination, reader.clone())?;
			}

			// Figure out which sub-protocol to send this packet to
			// - Should there be alternate handlers for 
			for &(id,ref handler) in PROTOCOL_HANDLDERS.read().iter()
			{
				if id == next_header {
					handler.dispatch(interface, hdr.source, hdr.destination, reader.clone());
				}
			}
			log_debug!("Unknown protocol {}", next_header);
			return Ok( () );
		}
	}
	Ok( () )
}

/// Register a protocol handler with this layer
pub fn register_handler(proto: u8, handler: fn(&Interface, Address, PacketReader)) -> Result<(), ()>
{
	let mut lh = PROTOCOL_HANDLDERS.write();
	for &(p, _) in lh.iter()
	{
		if p == proto {
			return Err( () );
		}
	}
	lh.push( (proto, ProtoHandler::DirectKernel(handler),) );
	Ok( () )
}

#[derive(PartialEq,Clone)]
struct HandlerKey {
	local: Address,
	remote: Address,
	mask: u8,
}
enum ProtoHandler
{
	/// Direct in-kernel handling (e.g. TCP)
	DirectKernel(fn(&Interface, Address, PacketReader)),
	/// Indirect user handling (pushes onto a buffer for the user to read from)
	// Ooh, another use for stack_dst, a DST queue!
	#[allow(dead_code)]
	User {
		key: HandlerKey,
		queue: ::kernel::lib::mem::Arc<::kernel::sync::Mutex< PacketQueue >>,
	},
}
impl ProtoHandler
{
	fn dispatch(&self, i: &Interface, src: Address, dest: Address, r: PacketReader)
	{
		match self
		{
		&ProtoHandler::DirectKernel(fcn) => fcn(i, src, r),
		ProtoHandler::User { key, queue} => {
			if dest == key.local && src.mask_net(key.mask) == key.remote {
				queue.lock().push(src, r);
			}
		},
		}
	}
}
pub struct PacketQueue {
	buf: ::kernel::lib::ring_buffer::RingBuf<u8>,
	waiters: ::kernel::lib::VecDeque<::kernel::threads::SleepObjectRef>,
}
impl PacketQueue {
	fn register_wait(&mut self, so: &::kernel::threads::SleepObject) {
		self.waiters.push_back(so.get_ref());
	}
	fn clear_wait(&mut self, so: &::kernel::threads::SleepObject) {
		self.waiters.retain(|v| {
			!v.is_from(so)
		})
	}
	fn has_packet(&self) -> bool {
		!self.buf.is_empty()
	}

	fn push(&mut self, src: Address, mut data: PacketReader) {
		let len = data.remain();
		let space = 16 + 4 + len;

		let lh = &mut self.buf;
		if lh.space() > space {
			for b in src.to_bytes() {
				let _ = lh.push_back(b);
			}
			for b in (len as u32).to_ne_bytes() {
				let _ = lh.push_back(b);
			}
			while let Ok(b) = data.read_u8() {
				let _ = lh.push_back(b);
			}

			if let Some(v) = self.waiters.pop_front() {
				v.signal();
			}
		}
	}
	fn pop(&mut self, buf: &mut [u8]) -> Option<(Address, usize)> {
		let lh = &mut self.buf;
		if lh.is_empty() {
			None
		}
		else {
			let a = Address::from_bytes({
				let mut b = [0; 16];
				for b in &mut b {
					*b = lh.pop_front().unwrap();
				}
				b
			});
			let len = u32::from_ne_bytes({
				let mut b = [0; 4];
				for b in &mut b {
					*b = lh.pop_front().unwrap();
				}
				b
			}) as usize;
			let read_len = usize::min(len, buf.len());
			for b in buf[..read_len].iter_mut() {
				*b = lh.pop_front().unwrap();
			}
			Some( (a, len) )
		}
	}
}

pub struct RawListenHandle {
	next_header: u8,
	key: HandlerKey,
	queue: ::kernel::lib::mem::Arc<::kernel::sync::Mutex< PacketQueue >>,
}
impl RawListenHandle {
	pub fn new(next_header: u8, source: Address, remote: (Address, u8)) -> Result<Self,()> {
		let key = HandlerKey { local: source, remote: remote.0, mask: remote.1 };
		let queue = ::kernel::lib::mem::Arc::new( ::kernel::sync::Mutex::new( PacketQueue {
			buf: ::kernel::lib::ring_buffer::RingBuf::new(1024*16),
			waiters: Default::default(),
		}));
		{
			let mut lh = PROTOCOL_HANDLDERS.write();
			for (p, h) in &*lh {
				if *p == next_header {
					if let ProtoHandler::User { key: cur_key, queue: _ } = h {
						if key == *cur_key {
							return Err(());
						}
					}
				}
			}
			lh.push((next_header, ProtoHandler::User { key: key.clone(), queue: queue.clone() }));
		}
		Ok(RawListenHandle {
			next_header,
			key,
			queue,
		})
	}

	pub fn register_wait(&self, so: &::kernel::threads::SleepObject) {
		self.queue.lock().register_wait(so)
	}
	pub fn clear_wait(&self, so: &::kernel::threads::SleepObject) {
		self.queue.lock().clear_wait(so);
	}
	pub fn has_packet(&self) -> bool {
		self.queue.lock().has_packet()
	}
	pub fn pop(&self, buf: &mut [u8]) -> Option<(Address, usize)> {
		self.queue.lock().pop(buf)
	}
}
impl Drop for RawListenHandle {
	fn drop(&mut self) {
		let mut lh = PROTOCOL_HANDLDERS.write();
		lh.retain(|(p,h)| {
			if *p == self.next_header {
				if let ProtoHandler::User { key: cur_key, queue: _ } = h {
					if *cur_key == self.key {
						return false;
					}
				}
			}
			true
		});
	}
}