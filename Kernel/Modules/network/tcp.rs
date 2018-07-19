// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/tcp.rs
//! Transmission Control Protocol (Layer 4)
use shared_map::SharedMap;
use kernel::prelude::*;
use kernel::lib::ring_buffer::RingBuf;

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

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq)]
enum Address
{
	Ipv4(::ipv4::Address),
}

static CONNECTIONS: SharedMap<(Address,u16,Address,u16), Connection> = SharedMap::new();
static PROTO_CONNECTIONS: SharedMap<(Address,u16,Address,u16), ProtoConnection> = SharedMap::new();
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

	// Options
	while pkt.remain() > pre_header_reader.remain() - hdr_len
	{
		match pkt.read_u8().unwrap()
		{
		_ => {},
		}
	}
	
	let quad = ( src_addr, hdr.source_port, dest_addr, hdr.dest_port, );
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
			if hdr.sequence_number == c.seen_seq + 1 && hdr.acknowlegement_number == c.sent_seq
			{
				// Make the full connection struct
				CONNECTIONS.insert(quad, Connection::new(&hdr));
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
			// TODO: Check the server's accept queue length
			//
			// - Add the quad as a proto-connection and send the SYN-ACK
			let pc = ProtoConnection::new(hdr.sequence_number);
			// TODO: Send the SYN-ACK
			PROTO_CONNECTIONS.insert(quad, pc);
		}
	}
	// Otherwise, drop
}

#[derive(Debug)]
struct PktHeader
{
	source_port: u16,
	dest_port: u16,
	sequence_number: u32,
	acknowlegement_number: u32,
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
const FLAG_ACK: u8 = 1 << 4;
impl PktHeader
{
	fn read(reader: &mut ::nic::PacketReader) -> Result<Self, ()>
	{
		Ok(PktHeader {
			source_port: reader.read_u16n()?,
			dest_port: reader.read_u16n()?,
			sequence_number: reader.read_u32n()?,
			acknowlegement_number: reader.read_u32n()?,
			data_offset: reader.read_u8()?,
			flags: reader.read_u8()?,
			window_size: reader.read_u16n()?,
			checksum: reader.read_u16n()?,
			urgent_pointer: reader.read_u16n()?,
			})
	}
	fn get_header_size(&self) -> usize {
		(self.data_offset >> 4) as usize * 4
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
			last_tx_seq: hdr.acknowlegement_number,
			tx_buffer: RingBuf::new(2048),
			}
	}
	fn handle(&self, hdr: &PktHeader, pkt: ::nic::PacketReader)
	{
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
}

