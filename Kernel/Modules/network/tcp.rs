// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/tcp.rs
//! Transmission Control Protocol (Layer 4)
use shared_map::SharedMap;
use kernel::sync::Mutex;
use kernel::lib::ring_buffer::AtomicRingBuf;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::nic::SparsePacket;
use crate::Address;
use kernel::futures::block_on;

const IPV4_PROTO_TCP: u8 = 6;


#[path="tcp-lib/"]
/// Library types just for TCP
mod lib {
	pub mod rx_buffer;
}

mod connection;
use self::connection::Connection;

fn earliest_timestamp(dst: &mut Option<::kernel::time::TickCount>, src: Option<::kernel::time::TickCount>) {
	//::kernel::log_trace!("{:?} < {:?}", dst, src);
	match src
	{
	Some(ts) => match *dst
		{
		Some(t2) if t2 < ts => {},
		_ => *dst = Some(ts),
		},
	None => {},
	}
}

pub fn init()
{
	crate::ipv4::register_handler(IPV4_PROTO_TCP, |int,src_addr, pkt| {
		rx_handler(Address::Ipv4(src_addr), Address::Ipv4(int.addr()), pkt)
	}).unwrap();
	crate::ipv6::register_handler(IPV4_PROTO_TCP, |int,src_addr, pkt| {
		rx_handler(Address::Ipv6(src_addr), Address::Ipv6(int.addr()), pkt)
	}).unwrap();

	// TODO: Spawn a worker that waits on all the TX timers and handles sending packets
	::core::mem::forget(::kernel::threads::WorkerThread::new("TCP Worker", || {
		// Check/advance all connections, also getting the timeout for the sleep
		loop
		{
			let key = WORKER_CV.get_key();
			let mut wakeup_time = None;
			for (quad, conn) in CONNECTIONS.iter()
			{
				earliest_timestamp(&mut wakeup_time, conn.lock().run_tasks(quad));
			}
			::kernel::log_trace!("wakeup_time = {:?}", wakeup_time);
			// Wait on a condvar with a timeout (based)
			// - This condvar will be poked when an incoming packet wants to trigger an action
			if let Some(wakeup_time) = wakeup_time {
				::kernel::futures::block_on(::kernel::futures::join_one(
					WORKER_CV.wait(key),
					::kernel::futures::msleep( (wakeup_time - ::kernel::time::ticks()) as usize )
					));
			}
			else {
				::kernel::futures::block_on(WORKER_CV.wait(key));
			}
		}
		}));
}

static CONNECTIONS: SharedMap<Quad, Mutex<Connection>> = SharedMap::new();
static PROTO_CONNECTIONS: SharedMap<Quad, ProtoConnection> = SharedMap::new();
static SERVERS: SharedMap<ListenPair, Server> = SharedMap::new();
static WORKER_CV: ::kernel::futures::Condvar = ::kernel::futures::Condvar::new();

static S_PORTS: Mutex<PortPool> = Mutex::new(PortPool::new());

/// Find the local source address for the given remote address
// TODO: Shouldn't this get an interface handle instead?
fn get_outbound_ip_for(addr: &Address) -> Option<Address>
{
	match addr
	{
	Address::Ipv4(addr) => crate::ipv4::route_lookup(crate::ipv4::Address::zero(), *addr).map(|r| Address::Ipv4(r.source_ip)),
	Address::Ipv6(addr) => crate::ipv6::route_lookup(crate::ipv6::Address::zero(), *addr).map(|r| Address::Ipv6(r.source_ip)),
	}
}
/// Allocate a port for the given local address
fn allocate_port(_addr: &Address) -> Option<u16>
{
	// TODO: Could store bitmap against the interface (having a separate bitmap for each interface)
	S_PORTS.lock().allocate()
}
fn release_port(_addr: &Address, idx: u16)
{
	S_PORTS.lock().release(idx)
}

fn rx_handler(src_addr: Address, dest_addr: Address, mut pkt: crate::nic::PacketReader)
{
	let pre_header_reader = pkt.clone();
	let hdr = match PktHeader::read(&mut pkt)
		{
		Ok(v) => v,
		Err(_) => {
			log_error!("Undersized packet: Ran out of data reading header");
			return ;
			},
		};
	log_debug!("hdr = {:?}", hdr);
	let hdr_len = hdr.get_header_size();
	if hdr_len > pre_header_reader.remain() {
		log_error!("Undersized or invalid packet: Header length is {} but packet length is {}", hdr_len, pre_header_reader.remain());
		return ;
	}

	// Validate checksum.
	let checksum = calculate_checksum(src_addr, dest_addr, &hdr, pkt.remain(),
	{
		let mut pkt = pkt.clone();
		let psum_whole = !crate::ipv4::calculate_checksum( (0 .. pkt.remain() / 2).map(|_| pkt.read_u16n().unwrap()) );
		// Final byte is decoded as if there was a zero after it (so as 0x??00)
		let psum_partial = if pkt.remain() > 0 { (pkt.read_u8().unwrap() as u16) << 8} else { 0 };
		crate::ipv4::calculate_checksum([psum_whole, psum_partial].iter().copied())
		}
		);
	if checksum != 0 {
		log_error!("Incorrect checksum: 0x{:04x} != 0", checksum);
		// TODO: Discard the packet.
	}

	// Options
	while pkt.remain() > pre_header_reader.remain() - hdr_len
	{
		match pkt.read_u8().unwrap()
		{
		_ => {},
		}
	}
	
	let get_server = ||->Option<_> {
		Option::or( SERVERS.get( &ListenPair::fixed(dest_addr, hdr.dest_port) ), SERVERS.get( &ListenPair::any(hdr.dest_port) ) )
		};

	let quad = Quad::new(dest_addr, hdr.dest_port, src_addr, hdr.source_port);
	// Search for active connections with this quad
	if let Some(c) = CONNECTIONS.get(&quad)
	{
		c.lock().handle(&quad, &hdr, pkt);
	}
	// Search for proto-connections
	// - Proto-connections are lighter weight than full-blown connections, reducing the impact of a SYN flood
	else if hdr.flags == FLAG_ACK
	{
		if let Some(c) = PROTO_CONNECTIONS.take(&quad)
		{
			// Check the SEQ/ACK numbers, and create the actual connection
			if hdr.sequence_number == c.seen_seq && hdr.acknowledgement_number == c.sent_seq.wrapping_add(1)
			{
				// Make the full connection struct
				match CONNECTIONS.insert(quad, Mutex::new(Connection::new_inbound(&hdr)))
				{
				Ok(()) => {
					log_debug!("Final ACK of a handshake: {:?}", quad);
					// Add the connection onto the server's accept queue
					let server = get_server().expect("Can't find server for proto connection");
					server.accept_queue.push(quad).expect("Acceped connection with full accept queue");
					// TODO: Signal a waiter too
					},
				Err(_) => log_warning!("Conflicting connection?"),	// TODO: What do to if there's a second connection for the quad?
				}
			}
			else
			{
				log_debug!("Bad ACK of a handshake: {:?} - SEQ {} != {} || ACK {} != {}", quad,
					hdr.sequence_number, c.seen_seq,
					hdr.acknowledgement_number, c.sent_seq.wrapping_add(1),
					);
				// - Bad ACK, put the proto connection back into the list
				let _ = PROTO_CONNECTIONS.insert(quad, c);
			}
		}
		else {
			// No proto connection - RST?
			log_debug!("Unexpected ACK: {:?}", quad);
			block_on(quad.send_packet(hdr.acknowledgement_number, hdr.sequence_number, FLAG_ACK|FLAG_RST, 0, &[], &[]));
		}
	}
	// If none found, look for servers on the destination (if SYN)
	else if hdr.flags & !FLAG_ACK == FLAG_SYN
	{
		if let Some(s) = get_server()
		{
			// Decrement the server's accept space
			if s.accept_space.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| if v == 0 { None } else { Some(v - 1) }).is_err() { 
				log_debug!("Start of incoming handshake: {:?} - Dropped, queue full", quad);
				// Reject if no space
				// - Send a RST
				// TODO: Queue a packet instead of blocking here
				block_on(quad.send_packet(hdr.acknowledgement_number, hdr.sequence_number.wrapping_add(1), FLAG_RST, 0, &[], &[]));
			}
			else {
				log_debug!("Start of incoming handshake: {:?}", quad);
				// - Add the quad as a proto-connection and send the SYN-ACK
				let pc = ProtoConnection::new(hdr.sequence_number.wrapping_add(1));
				block_on(quad.send_packet(pc.sent_seq, pc.seen_seq, FLAG_SYN|FLAG_ACK, hdr.window_size, &[], &[]));
				let _ = PROTO_CONNECTIONS.replace(quad, pc);	// Insert without replacing
			}
		}
		else
		{
			// Send a RST
			log_debug!("SYN to closed port: {:?}", quad);
			// All RSTs ACK the contents of the recieved packet
			let ack = hdr.sequence_number.wrapping_add( pre_header_reader.remain().max(1) as u32 );
			block_on(quad.send_packet(hdr.acknowledgement_number, ack, FLAG_RST|FLAG_ACK, 0, &[], &[]));
		}
	}
	// Otherwise, drop
}

fn calculate_checksum(src_addr: Address, dest_addr: Address, hdr: &PktHeader, tail_len: usize, tail_sum: u16) -> u16
{
	use crate::ipv4::calculate_checksum as ip_checksum;

	let packet_len = (5*4) + tail_len;

	// Pseudo header for checksum
	let sum_pseudo = match src_addr
		{
		Address::Ipv4(s) => {
			let Address::Ipv4(d) = dest_addr else { unreachable!() };
			ip_checksum([
				// Big endian stores MSB first, so write the high word first
				(s.as_u32() >> 16) as u16, (s.as_u32() >> 0) as u16,
				(d.as_u32() >> 16) as u16, (d.as_u32() >> 0) as u16,
				IPV4_PROTO_TCP as u16, packet_len as u16,
				].iter().copied())
			},
		Address::Ipv6(s) => {
			let Address::Ipv6(d) = dest_addr else { unreachable!() };
			ip_checksum([
				// Big endian stores MSB first, so write the high word first
				s.words()[0], s.words()[1], s.words()[2], s.words()[3],
				s.words()[4], s.words()[5], s.words()[6], s.words()[7],
				d.words()[0], d.words()[1], d.words()[2], d.words()[3],
				d.words()[4], d.words()[5], d.words()[6], d.words()[7],
				IPV4_PROTO_TCP as u16, packet_len as u16,
				].iter().copied())
			}
		};
	let sum_header = hdr.checksum();
	ip_checksum([ !sum_pseudo, !sum_header, !tail_sum ].iter().copied())
}

#[derive(Copy,Clone,PartialEq,PartialOrd,Eq,Ord)]
struct ListenPair(Option<Address>, u16);
impl ::core::fmt::Debug for ListenPair
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		f.write_str("Pair(")?;
		if let Some(ref a) = self.0 {
			a.fmt(f)?;
		}
		else {
			f.write_str("*")?;
		}
		write!(f, ":{})", self.1)
	}
}
impl ListenPair
{
	pub fn any(port: u16) -> ListenPair {
		ListenPair(None, port)
	}
	pub fn fixed(addr: Address, port: u16) -> ListenPair {
		ListenPair(Some(addr), port)
	}
}

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq)]
struct Quad
{
	local_addr: Address,
	local_port: u16,
	remote_addr: Address,
	remote_port: u16,
}
impl ::core::fmt::Debug for Quad
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "Quad({:?}:{} -> {:?}:{})", self.local_addr, self.local_port, self.remote_addr, self.remote_port)
	}
}
impl Quad
{
	fn new(local_addr: Address, local_port: u16, remote_addr: Address, remote_port: u16) -> Quad
	{
		Quad {
			local_addr, local_port, remote_addr, remote_port
			}
	}
	async fn send_packet(&self, seq: u32, ack: u32, flags: u8, window_size: u16, data1: &[u8], data2: &[u8])
	{
		// Make a header
		// TODO: Any options required?
		let options_bytes = &[];
		let opts_len_rounded = ((options_bytes.len() + 3) / 4) * 4;
		let hdr = {
			let mut hdr = PktHeader {
				source_port: self.local_port,
				dest_port: self.remote_port,
				sequence_number: seq,
				acknowledgement_number: ack,
				data_offset: ((5 + opts_len_rounded/4) << 4) as u8 | 0,
				flags,
				window_size,
				checksum: 0,	// To be filled afterwards
				urgent_pointer: 0,
				};
			hdr.checksum = calculate_checksum(
				self.local_addr, self.remote_addr,
				&hdr,
				data1.len()+data2.len(),
				super::ipv4::checksum::from_bytes(
					// TODO: options (padded to multiple of 4 bytes)
					Iterator::chain(data1.iter().copied(), data2.iter().copied() )
				)
				);
			hdr.as_bytes()
			};
		// Calculate checksum

		// Create sparse packet chain
		let data_pkt = SparsePacket::new_root(data2);
		let data_pkt = SparsePacket::new_chained(data1, &data_pkt);
		// - Padding required to make the header a multiple of 4 bytes long
		let opt_pad_pkt = SparsePacket::new_chained(&[0; 3][.. opts_len_rounded - options_bytes.len()], &data_pkt);
		let opt_pkt = SparsePacket::new_chained(options_bytes, &opt_pad_pkt);
		let hdr_pkt = SparsePacket::new_chained(&hdr, &opt_pkt);

		// Pass packet downstream
		match match self.local_addr
			{
			Address::Ipv4(a) => crate::ipv4::send_packet(a, self.remote_addr.unwrap_ipv4(), IPV4_PROTO_TCP, hdr_pkt).await,
			Address::Ipv6(a) => crate::ipv6::send_packet(a, self.remote_addr.unwrap_ipv6(), IPV4_PROTO_TCP, hdr_pkt).await,
			}
		{
		Ok(()) => {},
		Err(_) => {
			// TODO: Propagate error? This is NoRoute
		},
		}
	}
}

#[derive(Debug)]
struct PktHeader
{
	source_port: u16,
	dest_port: u16,
	sequence_number: u32,
	acknowledgement_number: u32,
	/// Packed: top 4 bits are header size in 4byte units, bottom 4 are reserved
	data_offset: u8,
	/// Bitfield:
	/// 0: FIN
	/// 1: SYN
	/// 2: RST
	/// 3: PSH
	/// 4: ACK
	/// 5: URG
	/// 6: ECE
	/// 7: CWR
	flags: u8,
	window_size: u16,

	checksum: u16,
	urgent_pointer: u16,

	//options: [u8],
}
const FLAG_FIN: u8 = 1 << 0;
const FLAG_SYN: u8 = 1 << 1;
const FLAG_RST: u8 = 1 << 2;
const FLAG_PSH: u8 = 1 << 3;
const FLAG_ACK: u8 = 1 << 4;
impl PktHeader
{
	fn read(reader: &mut crate::nic::PacketReader) -> Result<Self, ()>
	{
		Ok(PktHeader {
			source_port: reader.read_u16n()?,
			dest_port: reader.read_u16n()?,
			sequence_number: reader.read_u32n()?,
			acknowledgement_number: reader.read_u32n()?,
			data_offset: reader.read_u8()?,
			flags: reader.read_u8()?,
			window_size: reader.read_u16n()?,
			checksum: reader.read_u16n()?,
			urgent_pointer: reader.read_u16n()?,
			})
		// TODO: Check checksum?
	}
	fn get_header_size(&self) -> usize {
		(self.data_offset >> 4) as usize * 4
	}

	fn as_bytes(&self) -> [u8; 5*4]
	{
		[
			(self.source_port >> 8) as u8,
			(self.source_port >> 0) as u8,
			(self.dest_port >> 8) as u8,
			(self.dest_port >> 0) as u8,
			(self.sequence_number >> 24) as u8,
			(self.sequence_number >> 16) as u8,
			(self.sequence_number >> 8) as u8,
			(self.sequence_number >> 0) as u8,
			(self.acknowledgement_number >> 24) as u8,
			(self.acknowledgement_number >> 16) as u8,
			(self.acknowledgement_number >> 8) as u8,
			(self.acknowledgement_number >> 0) as u8,
			self.data_offset,
			self.flags,
			(self.window_size >> 8) as u8,
			(self.window_size >> 0) as u8,
			(self.checksum >> 8) as u8,
			(self.checksum >> 0) as u8,
			(self.urgent_pointer >> 8) as u8,
			(self.urgent_pointer >> 0) as u8,
			]
	}
	fn checksum(&self) -> u16 {
		crate::ipv4::checksum::from_bytes(self.as_bytes().iter().copied())
	}
}


struct ProtoConnection
{
	seen_seq: u32,
	sent_seq: u32,
}
impl ProtoConnection
{
	fn new(seen_seq: u32) -> ProtoConnection
	{
		ProtoConnection {
			seen_seq: seen_seq,
			sent_seq: 1,	// TODO: Random
			}
	}
}

struct Server
{
	// Amount of connections that can still be accepted
	accept_space: AtomicUsize,
	// Established connections waiting for the user to accept
	accept_queue: AtomicRingBuf<Quad>,
}

#[derive(Debug)]
pub enum ListenError
{
	SocketInUse,
}

/// A handle to a LISTEN socket
pub struct ServerHandle(ListenPair);
impl ServerHandle
{
	pub fn listen(port: u16) -> Result<ServerHandle,ListenError>
	{
		let p = ListenPair::any(port);
		SERVERS.insert(p, Server {
			accept_space: AtomicUsize::new(10),
			accept_queue: AtomicRingBuf::new(10),
			}).map_err(|_| ListenError::SocketInUse)?;
		Ok( ServerHandle(p) )
	}

	/// Accept a new incoming connection
	pub fn accept(&self) -> Option<ConnectionHandle>
	{
		let s = SERVERS.get(&self.0).expect("Server entry missing while handle still exists");
		let rv_quad = s.accept_queue.pop()?;
		Some( ConnectionHandle(rv_quad) )
	}

	//pub fn wait_accept(&mut self)
}

/// Handle to an open (or partially-open) connection
/// 
/// Can be directly constructed (for an outgoing/client connection), or returned from a server
pub struct ConnectionHandle(Quad);

#[derive(Debug)]
pub enum ConnError
{
	NoRoute,
	LocalClosed,
	TimedOut,
	RemoteRefused,
	RemoteClosed,
	RemoteReset,
	NoPortAvailable,
}

impl ConnectionHandle
{
	pub fn connect(addr: Address, port: u16) -> Result<ConnectionHandle, ConnError>
	{
		log_trace!("ConnectionHandle::connect({:?}, {})", addr, port);
		// 1. Determine the local address for this remote address
		let local_addr = match get_outbound_ip_for(&addr)
			{
			Some(a) => a,
			None => return Err(ConnError::NoRoute),
			};
		// 2. Pick a local port
		let local_port = match allocate_port(&local_addr)
			{
			Some(p) => p,
			None => return Err(ConnError::NoPortAvailable),
			};
		// 3. Create the quad and allocate the connection structure
		let quad = Quad::new(local_addr, local_port,  addr, port, );
		log_trace!("ConnectionHandle::connect: quad={:?}", quad);
		// 4. Send the opening SYN (by creating the outbound connection structure)
		let conn = Connection::new_outbound(&quad, 0x10000u32);
		CONNECTIONS.insert(quad, Mutex::new(conn)).map_err(|_| ()).expect("Our unique port wasn't unique");
		WORKER_CV.wake_one();
		Ok( ConnectionHandle(quad) )
	}

	pub fn remote_addr(&self) -> (super::Address, u16) {
		(self.0.remote_addr, self.0.remote_port)
	}

	fn conn(&self) -> shared_map::Handle<'_, Quad,Mutex<Connection>> {
		match CONNECTIONS.get(&self.0)
		{
		None => panic!("Connection {:?} removed before handle dropped", self.0),
		Some(v) => v,
		}
	}

	pub fn bind_wait_connected(&self, obj: &::kernel::threads::SleepObject) {
		self.conn().lock().connected_wait_bind(obj)
	}
	pub fn unbind_wait_connected(&self, obj: &::kernel::threads::SleepObject) -> bool {
		self.conn().lock().connected_wait_unbind(obj)
	}
	pub fn is_connected(&self) -> bool {
		self.conn().lock().connection_complete()
	}

	pub fn send_data(&self, buf: &[u8]) -> Result<usize, ConnError> {
		self.conn().lock().send_data(&self.0, buf)
	}
	pub fn bind_wait_send(&self, obj: &::kernel::threads::SleepObject) {
		self.conn().lock().send_wait_bind(obj)
	}
	pub fn unbind_wait_send(&self, obj: &::kernel::threads::SleepObject) -> bool {
		self.conn().lock().send_wait_unbind(obj)
	}
	pub fn send_ready(&self) -> bool {
		self.conn().lock().send_ready()
	}

	pub fn recv_data(&self, buf: &mut [u8]) -> Result<usize, ConnError> {
		self.conn().lock().recv_data(&self.0, buf)
	}
	pub fn bind_wait_recv(&self, obj: &::kernel::threads::SleepObject) {
		self.conn().lock().recv_wait_bind(obj)
	}
	pub fn unbind_wait_recv(&self, obj: &::kernel::threads::SleepObject) -> bool {
		self.conn().lock().recv_wait_unbind(obj)
	}
	pub fn recv_ready(&self) -> bool {
		self.conn().lock().recv_ready()
	}

	pub fn close(&mut self) -> Result<(), ConnError> {
		self.conn().lock().close(&self.0)
	}
}
impl ::core::ops::Drop for ConnectionHandle
{
	fn drop(&mut self)
	{
		// Mark the connection to close
	}
}


const MIN_DYN_PORT: u16 = 0xC000;
const N_DYN_PORTS: usize = (1<<16) - MIN_DYN_PORT as usize;
struct PortPool {
	bitmap: [u32; N_DYN_PORTS / 32],
	//n_free_ports: u16,
	next_port: u16,
}
impl PortPool
{
	const fn new() -> PortPool
	{
		PortPool {
			bitmap: [0; N_DYN_PORTS / 32],
			//n_free_ports: N_DYN_PORTS as u16,
			next_port: MIN_DYN_PORT,
			}
	}

	fn ofs_mask(idx: u16) -> Option<(usize, u32)>
	{
		if idx >= MIN_DYN_PORT
		{
			let ofs = (idx - MIN_DYN_PORT) as usize / 32;
			let mask  = 1 << (idx % 32);
			Some( (ofs, mask) )
		}
		else
		{
			None
		}
	}
	fn take(&mut self, idx: u16) -> Result<(),()>
	{
		let (ofs,mask) = match Self::ofs_mask(idx)
			{
			Some(v) => v,
			None => return Ok(()),
			};
		if self.bitmap[ofs] & mask != 0 {
			Err( () )
		}
		else {
			self.bitmap[ofs] |= mask;
			Ok( () )
		}
	}
	fn release(&mut self, idx: u16)
	{
		let (ofs,mask) = match Self::ofs_mask(idx)
			{
			Some(v) => v,
			None => return,
			};
		self.bitmap[ofs] &= !mask;
	}
	fn allocate(&mut self) -> Option<u16>
	{
		// Strategy: Linear ('cos it's easy)
		for idx in self.next_port ..= 0xFFFF
		{
			match self.take(idx)
			{
			Ok(_) => { self.next_port = idx; return Some(idx); },
			_ => {},
			}
		}
		for idx in MIN_DYN_PORT .. self.next_port
		{
			match self.take(idx)
			{
			Ok(_) => { self.next_port = idx; return Some(idx); },
			_ => {},
			}
		}
		None
	}
}

