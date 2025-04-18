//! UDP handling
use kernel::lib::ring_buffer::RingBuf;
use kernel::sync::{RwLock,Mutex};
use kernel::lib::mem::Arc;
use kernel::vec::Vec;
use crate::Address;
use crate::nic::SparsePacket;

const IPV4_PROTO_UDP: u8 = 17;
/// Opened sockets
static SOCKETS: RwLock<Vec< Arc<SocketInfo> >> = RwLock::new(Vec::new());


pub fn init() {
	crate::ipv4::register_handler(IPV4_PROTO_UDP, |int,src_addr,pkt|{
		rx_handler(Address::Ipv4(src_addr), Address::Ipv4(int.addr()), pkt)
	}).unwrap();
}
fn rx_handler(src_addr: Address, dst_addr: Address, mut pkt: crate::nic::PacketReader)
{
	let hdr = match PktHeader::read(&mut pkt)
		{
		Ok(v) => v,
		Err(_) => {
			log_error!("Undersized packet: Ran out of data reading header");
			return ;
			},
		};
	// Check checksum.
	let cksum = calc_checksum(
		&hdr.encode(),
		&src_addr, &dst_addr, pkt.remain(),
		{ let mut p = pkt.clone(); ::core::iter::from_fn(move || p.read_u8().ok()) }
	);
	if cksum != 0 {
	}
	let pkt_data = pkt.clone();
	for sock in SOCKETS.read().iter() {
		if sock.key.local_port != hdr.dest_port {
			continue ;
		}
		match sock.key.local_address {
		Some(a) if a != dst_addr => continue,
		_ => {},
		}
		if sock.key.remote_mask.0 != src_addr.mask_network(sock.key.remote_mask.1) {
			continue ;
		}
		match sock.key.remote_port {
		Some(a) if a != hdr.source_port => continue,
		_ => {},
		}
		// Matches!
		sock.rx_buffer.push_packet(dst_addr, src_addr, hdr.source_port, pkt_data.clone());
	}
}

pub enum Error {
	/// Cannot create a new socket, the chosen local address is in use
	AddressInUse,
	/// Trying to call `send` without a concrete destination
	UnboundSocket,
	/// Sending to a remote address that isn't in the recieve mask
	InvalidRemote,
	/// Trying to send to an address of a different type to the existing bound address
	IncompatibleAddresses,
}

/// Exposed handle to a registered socket
pub struct SocketHandle {
	inner: Arc<SocketInfo>,
}
impl SocketHandle {
	pub fn new(
		local_address: Option<crate::Address>,
		local_port: u16,
		remote_mask: (crate::Address, u8),
		remote_port: Option<u16>,
	) -> Result<Self, Error> {
		if local_port == 0 {
			todo!("Allocate a local port");
		}
		let key = SocketKey {
			local_address,
			local_port,
			remote_mask,
			remote_port,
		};
		// Check for an overlapping socket
		let mut lh = SOCKETS.write();
		for sock in lh.iter() {
			if sock.key.overlaps_with(&key) {
				return Err(Error::AddressInUse);
			}
		}
		let rv = Arc::new(SocketInfo {
			key,
			rx_buffer: Default::default(),
		});
		lh.push(rv.clone());
		Ok(SocketHandle { inner: rv })
	}
	pub fn try_recv_from(&mut self, buf: &mut [u8]) -> Option<(usize, Address, u16)> {
		match self.inner.rx_buffer.pop_packet(buf) {
		None => None,
		Some((_dst,src,port,len)) => {
			Some((len, src, port))
		}
		}
	}
	/// Send a datagram to the single target address
	pub fn send(&self, buf: SparsePacket) -> Result<(),Error> {
		match (self.inner.key.remote_mask, self.inner.key.remote_port) {
		((addr,bits), Some(port)) if addr.mask_network(bits) == addr => {
			self.send_to(addr, port, buf)
		}
		_ => Err(Error::UnboundSocket)
		}
	}
	/// Send a datagram over this socket
	pub fn send_to(&self, addr: Address, port: u16, buf: SparsePacket) -> Result<(),Error> {
		// Check if the target address matches the remote mask
		// TODO: Is this actually needed/right?
		let (d_addr,bits) = self.inner.key.remote_mask;
		if addr.mask_network(bits) != d_addr.mask_network(bits) {
			return Err(Error::InvalidRemote);
		}
		match self.inner.key.remote_port {
		None => {},
		Some(v) if v == port => {},
		_ => return Err(Error::InvalidRemote),
		}

		// Create header
		let mut hdr = PktHeader {
			source_port: self.inner.key.local_port,
			dest_port: port,
			length: buf.total_len() as u16,
			checksum: 0,
		};
		let local_addr = match self.inner.key.local_address {
			Some(a) => a,
			None => todo!(),
		};
		// - incl. checksum (if no offload)
		if true {
			hdr.checksum = calc_checksum(
				&hdr.encode(),
				&local_addr, &addr, buf.total_len(),
				buf.into_iter().map(|v| v.iter().copied()).flatten()
			);
		}
		let hdr_enc = hdr.encode();
		let pkt = SparsePacket::new_chained(&hdr_enc, &buf);
		// Send
		match addr {
		Address::Ipv4(dest) => {
			let Address::Ipv4(source) = local_addr else { return Err(Error::IncompatibleAddresses) };
			kernel::futures::block_on(crate::ipv4::send_packet(source, dest, IPV4_PROTO_UDP, pkt));
		}
		}
		Ok( () )
	}
}
impl ::core::ops::Drop for SocketHandle {
	fn drop(&mut self) {
		let mut lh = SOCKETS.write();
		lh.retain(|v| !Arc::ptr_eq(v, &self.inner));
	}
}
/// Underlying information on an open/listening socket
struct SocketInfo {
	key: SocketKey,
	rx_buffer: MessagePool,
}
/// The key part of a socket
struct SocketKey {
	local_address: Option<crate::Address>,
	local_port: u16,
	remote_mask: (crate::Address, u8),
	remote_port: Option<u16>,
}
impl SocketKey {
	fn overlaps_with(&self, other: &SocketKey) -> bool {
		// Local port: if the local port is different, then this cannot overlap
		if self.local_port != other.local_port {
			return false;
		}
		// Remote port: If both are `Some` but different, then no overlap - otherwise possible.
		match (self.remote_port, other.remote_port) {
		(Some(a),Some(b)) if a != b => return false,
		_ => {},
		}
		// Local address: Same as remote port... but is overlap allowed?
		match (self.local_address, other.local_address) {
		(Some(a),Some(b)) if a != b => return false,
		_ => {},
		}
		// Remote: Check if the spans covered by the mask are disjoint.
		let min_bits = u8::min(self.remote_mask.1, other.remote_mask.1);
		self.remote_mask.0.mask_network(min_bits) == other.remote_mask.0.mask_network(min_bits)
	}
}
/// A pool of messages, stored as u16 length-delimited data in a ring-buf
struct MessagePool {
	inner: Mutex<RingBuf<u8>>,
}
impl Default for MessagePool {
	fn default() -> Self {
		Self { inner: Mutex::new(RingBuf::new(1024*32)) }
	}
}
impl MessagePool {
	fn push_packet(&self, dest_addr: Address, src_addr: Address, src_port: u16, mut pkt: crate::nic::PacketReader) {
		// Header:
		// - port: u16
		// - pkt_len: u16
		// - addr_ty: u8
		// - _pad: u8
		// - address data
		fn make_hdr<'a>(dst: &'a mut [u8], len: usize, src_port: u16, aty: u8, dest_addr: &[u8], src_addr: &[u8]) -> &'a [u8] {
			dst[0..][..2].copy_from_slice(&(len as u16).to_ne_bytes());
			dst[2..][..2].copy_from_slice(&src_port.to_ne_bytes());
			dst[4..][..1].copy_from_slice(&aty.to_ne_bytes());
			dst[5..][..1].copy_from_slice(&0u8.to_ne_bytes());
			dst[6..][..dest_addr.len()].copy_from_slice(dest_addr);
			dst[6+dest_addr.len()..][..src_addr.len()].copy_from_slice(src_addr);
			&dst[..6+dest_addr.len()+src_addr.len()]
		}
		let mut hdr_buf = [0; 3*2 + 16*2];
		let len = pkt.remain();
		let hdr = match (dest_addr,src_addr) {
			(Address::Ipv4(dest_addr), Address::Ipv4(src_addr)) => {
				make_hdr(&mut hdr_buf, len, src_port, 0, &dest_addr.0, &src_addr.0)
			}
		};

		let mut lh = self.inner.lock();
		if lh.space() < hdr.len() + pkt.remain() {
		}
		else {
			for &b in hdr {
				let _ = lh.push_back(b);
			}
			while let Ok(b) = pkt.read_u8() {
				let _ = lh.push_back(b);
			}
		}
		//self.inner.push(val)
	}
	fn pop_packet(&self, buf: &mut [u8]) -> Option<(Address, Address, u16, usize)> {
		let mut lh = self.inner.lock();
		if lh.len() == 0 {
			None
		}
		else {
			assert!(lh.len() > 6);
			// Get common header
			let len = u16::from_ne_bytes([ lh.pop_front().unwrap(), lh.pop_front().unwrap()]) as usize;
			let port = u16::from_ne_bytes([ lh.pop_front().unwrap(), lh.pop_front().unwrap()]);
			let aty = lh.pop_front().unwrap();
			let _pad = lh.pop_front().unwrap();
			// Get addresses
			let (dst,src) = match aty {
				0 => {	// IPv4
					let da = [ lh.pop_front().unwrap(), lh.pop_front().unwrap(),lh.pop_front().unwrap(), lh.pop_front().unwrap(), ];
					let sa = [ lh.pop_front().unwrap(), lh.pop_front().unwrap(),lh.pop_front().unwrap(), lh.pop_front().unwrap(), ];
					(
						Address::Ipv4(crate::ipv4::Address(da)),
						Address::Ipv4(crate::ipv4::Address(sa)),
					)
				}
				_ => panic!("Unknown address type in packet queue"),
				};
			// Get data
			assert!(lh.len() > len);
			for dst in buf.iter_mut().take(len) {
				*dst = lh.pop_front().unwrap();
			}
			Some((dst, src, port, len))
		}
	}
}


#[derive(Debug)]
struct PktHeader
{
	source_port: u16,
	dest_port: u16,
	length: u16,
	checksum: u16,
}
impl PktHeader
{
	fn read(reader: &mut crate::nic::PacketReader) -> Result<Self, ()>
	{
		Ok(PktHeader {
			source_port: reader.read_u16n()?,
			dest_port: reader.read_u16n()?,
			length: reader.read_u16n()?,
			checksum: reader.read_u16n()?,
			})
	}
	fn encode(&self) -> [u8; 8] {
		// SAFE: 
		unsafe { ::core::mem::transmute([
			self.source_port.to_le_bytes(),
			self.dest_port.to_le_bytes(),
			self.length.to_le_bytes(),
			self.checksum.to_le_bytes(),
		]) }
	}
}
fn calc_checksum(hdr: &[u8], src_addr: &Address, dst_addr: &Address, data_len: usize, data: impl Iterator<Item=u8>) -> u16 {
	let pkt_len = ((hdr.len() + data_len) as u16).to_be_bytes();
	match src_addr {
	Address::Ipv4(src_addr) => {
		let Address::Ipv4(dst_addr) = dst_addr else { panic!("Mismatched address types") };
		let ph = [
			src_addr.0[0], src_addr.0[1], src_addr.0[2], src_addr.0[3],
			dst_addr.0[0], dst_addr.0[1], dst_addr.0[2], dst_addr.0[3],
			0, IPV4_PROTO_UDP,
			pkt_len[0],pkt_len[1],
		];
		calc_checksum_inner(hdr, &ph, data)
	},
	}
}
fn calc_checksum_inner(hdr: &[u8], ph: &[u8], data: impl Iterator<Item=u8>) -> u16 
{
	return super::ipv4::calculate_checksum(Words(
		Iterator::chain(ph.iter().copied(), hdr.iter().copied())
		.chain( data )
	));
	struct Words<I>(I);
	impl<I> Iterator for Words<I>
	where I: Iterator<Item=u8>
	{
		type Item = u16;
		
		fn next(&mut self) -> Option<Self::Item> {
			// NOTE: This only really works on fused iterators
			match (self.0.next(),self.0.next()) {
			(Some(a),Some(b)) => Some(u16::from_be_bytes([a,b])),
			(Some(a),None) => Some(u16::from_be_bytes([a,0])),
			(None,_) => None,
			}
		}
	}
}