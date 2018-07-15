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

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq)]
enum Address
{
	Ipv4(::ipv4::Address),
}

static CONNECTIONS: SharedMap<(Address,u16,Address,u16), Connection> = SharedMap::new();
static PROTO_CONNECTIONS: SharedMap<(Address,u16,Address,u16), ProtoConnection> = SharedMap::new();
static SERVERS: SharedMap<(Option<Address>,u16), Server> = SharedMap::new();

fn rx_handler_v4(int: &::ipv4::Interface, src_addr: ::ipv4::Address, mut pkt: ::nic::PacketReader)
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

struct RxBuffer
{
	// Number of bytes in the buffer
	// Equal to `8 * data.len() / 9`
	size: usize,
	// Start of the first non-consumed byte
	read_pos: usize,
	// Bitmap followed by data
	data: Vec<u8>,
}
impl RxBuffer
{
	fn new(window_size: usize) -> RxBuffer {
		let mut rv = RxBuffer {
			size: 0, read_pos: 0, data: vec![],
			};
		rv.resize(window_size);
		rv
	}
	fn insert(&mut self, offset: usize, data: &[u8]) {
	}
	fn take(&mut self, buf: &mut [u8]) -> usize {
		let out_len = ::core::cmp::min( buf.len(), self.valid_len() );
		panic!("TODO");
	}
	fn valid_len(&self) -> usize
	{
		// Number of valid bytes in the first partial bitmap entry
		let mut len = {
			let ofs = self.read_pos % 8;
			let v = self.data[self.size..][self.read_pos/8] >> ofs;
			(!v).trailing_zeros()
			};
		if len > 0
		{
			for i in 1 .. self.size / 8
			{
				let v = self.data[self.size ..][i];
				if v != 0xFF
				{
					len += (!v).trailing_zeros();
					break;
				}
				else
				{
					len += 8;
				}
			}
			// NOTE: There's an edge case where if the buffer is 100% full, it won't return that (if the read position is unaligned)
			// But that isn't a critical problem.
		}
		len as usize
	}
	fn resize(&mut self, new_size: usize) {
		self.compact();
		assert!(self.read_pos == 0);
		if new_size > self.size {
			// Resize underlying vector
			self.data.resize(new_size + (new_size + 7) / 8, 0u8);
			// Copy/move the bitmap up
			self.data[self.size ..].rotate_right( (new_size - self.size) / 8 );
		}
		else {
			// Move the bitmap down
			self.data[new_size ..].rotate_left( (self.size - new_size) / 8 );
			self.data.truncate( new_size + (new_size + 7) / 8 );
		}
		self.size = new_size;
	}
	/// Compact the current state so read_pos=0
	fn compact(&mut self)
	{
		if self.read_pos != 0
		{
			// Rotate data
			self.data[..self.size].rotate_left( self.read_pos );
			// Bitmap:
			// Step 1: Octet align
			let bitofs = self.read_pos % 8;
			if bitofs > 0
			{
				let bitmap_rgn = &mut self.data[self.size ..];
				let mut last_val = bitmap_rgn[0];
				for p in bitmap_rgn.iter_mut().rev()
				{
					let v = (last_val << (8 - bitofs)) | (*p >> bitofs);;
					last_val = ::core::mem::replace(p, v);
				}
			}
			// Step 2: shift bytes down
			self.data[self.size ..].rotate_left( self.read_pos / 8 );
		}
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

