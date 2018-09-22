// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/tcp.rs
//! Transmission Control Protocol (Layer 4)
use shared_map::SharedMap;
use kernel::prelude::*;
use kernel::lib::ring_buffer::{RingBuf,AtomicRingBuf};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::nic::SparsePacket;

pub fn init()
{
	::ipv4::register_handler(6, rx_handler_v4);
}

#[path="tcp-lib/"]
/// Library types just for TCP
mod lib {
	pub mod rx_buffer;
}
use self::lib::rx_buffer::RxBuffer;

static CONNECTIONS: SharedMap<Quad, Connection> = SharedMap::new();
static PROTO_CONNECTIONS: SharedMap<Quad, ProtoConnection> = SharedMap::new();
static SERVERS: SharedMap<(Option<Address>,u16), Server> = SharedMap::new();


fn rx_handler_v4(int: &::ipv4::Interface, src_addr: ::ipv4::Address, pkt: ::nic::PacketReader)
{
	rx_handler(Address::Ipv4(src_addr), Address::Ipv4(int.addr()), pkt)
}
fn rx_handler(src_addr: Address, dest_addr: Address, mut pkt: ::nic::PacketReader)
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
	if hdr_len < pre_header_reader.remain() {
		log_error!("Undersized or invalid packet: Header length is {} but packet length is {}", hdr_len, pre_header_reader.remain());
		return ;
	}

	// TODO: Validate checksum.
	{
		let sum_header = hdr.checksum();
		let sum_options = {
			let mut pkt = pkt.clone();
			::ipv4::calculate_checksum( (0 .. (pre_header_reader.remain() - hdr_len) / 2).map(|_| pkt.read_u16n().unwrap()) )
			};
		//let sum_data = ::ipv4::calculate_checksum( pkt.iter_u16n() );
	}

	// Options
	while pkt.remain() > pre_header_reader.remain() - hdr_len
	{
		match pkt.read_u8().unwrap()
		{
		_ => {},
		}
	}
	
	let quad = Quad::new(dest_addr, hdr.dest_port, src_addr, hdr.source_port);
	// Search for active connections with this quad
	if let Some(c) = CONNECTIONS.get(&quad)
	{
		c.handle(&hdr, pkt);
	}
	// Search for proto-connections
	// - Proto-connections are lighter weight than full-blown connections, reducing the impact of a SYN flood
	else if hdr.flags == FLAG_ACK
	{
		if let Some(c) = PROTO_CONNECTIONS.take(&quad)
		{
			// Check the SEQ/ACK numbers, and create the actual connection
			if hdr.sequence_number == c.seen_seq + 1 && hdr.acknowledgement_number == c.sent_seq
			{
				// Make the full connection struct
				CONNECTIONS.insert(quad, Connection::new(&hdr));
				// Add the connection onto the server's accept queue
				let server = Option::or( SERVERS.get( &(Some(dest_addr), hdr.dest_port) ), SERVERS.get( &(None, hdr.dest_port) ) ).expect("Can't find server");
				server.accept_queue.push(quad).expect("Acceped connection with full accept queue");
			}
			else
			{
				// - Bad ACK, put the proto connection back into the list
				PROTO_CONNECTIONS.insert(quad, c);
			}
		}
	}
	// If none found, look for servers on the destination (if SYN)
	else if hdr.flags == FLAG_SYN
	{
		if let Some(s) = Option::or( SERVERS.get( &(Some(dest_addr), hdr.dest_port) ), SERVERS.get( &(None, hdr.dest_port) ) )
		{
			// Decrement the server's accept space
			if s.accept_space.fetch_update(|v| if v == 0 { None } else { Some(v - 1) }, Ordering::SeqCst, Ordering::SeqCst).is_err() { 
				// Reject if no space
				// - Send a RST
				quad.send_packet(hdr.acknowledgement_number, hdr.sequence_number, FLAG_RST, 0, &[]);
			}
			else {
				// - Add the quad as a proto-connection and send the SYN-ACK
				let pc = ProtoConnection::new(hdr.sequence_number);
				quad.send_packet(pc.sent_seq, pc.seen_seq, FLAG_SYN|FLAG_ACK, hdr.window_size, &[]);
				PROTO_CONNECTIONS.insert(quad, pc);
			}
		}
		else
		{
			// Send a RST
			quad.send_packet(hdr.acknowledgement_number, hdr.sequence_number, FLAG_RST, 0, &[]);
		}
	}
	// Otherwise, drop
}

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq,Debug)]
enum Address
{
	Ipv4(::ipv4::Address),
}
impl Address
{
	fn unwrap_ipv4(&self) -> ::ipv4::Address {
		match self {
		&Address::Ipv4(v) => v,
		}
	}
}

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq,Debug)]
struct Quad
{
	local_addr: Address,
	local_port: u16,
	remote_addr: Address,
	remote_port: u16,
}
impl Quad
{
	fn new(local_addr: Address, local_port: u16, remote_addr: Address, remote_port: u16) -> Quad
	{
		Quad {
			local_addr, local_port, remote_addr, remote_port
			}
	}
	fn send_packet(&self, seq: u32, ack: u32, flags: u8, window_size: u16, data: &[u8])
	{
		// Make a header
		// TODO: Any options required?
		let options_bytes = &[];
		let opts_len_rounded = ((options_bytes.len() + 3) / 4) * 4;
		let hdr = PktHeader {
			source_port: self.local_port,
			dest_port: self.remote_port,
			sequence_number: seq,
			acknowledgement_number: ack,
			data_offset: ((5 + opts_len_rounded/4) << 4) as u8 | 0,
			flags: flags,
			window_size: window_size,
			checksum: 0,	// To be filled afterwards
			urgent_pointer: 0,
			}.as_bytes();
		// Calculate checksum

		// Create sparse packet chain
		let data_pkt = SparsePacket::new_root(data);
		// - Padding required to make the header a multiple of 4 bytes long
		let opt_pad_pkt = SparsePacket::new_chained(&[0; 3][.. opts_len_rounded - options_bytes.len()], &data_pkt);
		let opt_pkt = SparsePacket::new_chained(options_bytes, &opt_pad_pkt);
		let hdr_pkt = SparsePacket::new_chained(&hdr, &opt_pkt);

		// Pass packet downstream
		match self.local_addr
		{
		Address::Ipv4(a) => ::ipv4::send_packet(a, self.remote_addr.unwrap_ipv4(), hdr_pkt),
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
const FLAG_SYN: u8 = 1 << 1;
const FLAG_RST: u8 = 1 << 2;
const FLAG_ACK: u8 = 1 << 4;
impl PktHeader
{
	fn read(reader: &mut ::nic::PacketReader) -> Result<Self, ()>
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
	fn as_u16s(&self) -> [u16; 5*2] {
		[
			self.source_port,
			self.dest_port,
			(self.sequence_number >> 16) as u16,
			(self.sequence_number >> 0) as u16,
			(self.acknowledgement_number >> 16) as u16,
			(self.acknowledgement_number >> 0) as u16,
			(self.data_offset as u16) << 8 | (self.flags as u16),
			self.window_size,
			self.checksum,
			self.urgent_pointer,
			]
	}
	fn checksum(&self) -> u16 {
		::ipv4::calculate_checksum(self.as_u16s().iter().cloned())
	}
}

struct Connection
{
	/// Sequence number of the next expected remote byte
	next_rx_seq: u32,
	/// Received bytes
	rx_buffer: RxBuffer,

	/// Sequence number of last transmitted byte
	last_tx_seq: u32,
	/// Buffer of transmitted but not ACKed bytes
	tx_buffer: RingBuf<u8>,
}
impl Connection
{
	fn new(hdr: &PktHeader) -> Self
	{
		Connection {
			next_rx_seq: hdr.sequence_number,
			rx_buffer: RxBuffer::new(2048),
			last_tx_seq: hdr.acknowledgement_number,
			tx_buffer: RingBuf::new(2048),
			}
	}
	fn handle(&self, hdr: &PktHeader, pkt: ::nic::PacketReader)
	{
		// TODO: Handle various stages of a connection
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

