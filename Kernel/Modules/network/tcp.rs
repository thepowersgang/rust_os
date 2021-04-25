// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/tcp.rs
//! Transmission Control Protocol (Layer 4)
use shared_map::SharedMap;
use kernel::sync::Mutex;
use kernel::lib::ring_buffer::{RingBuf,AtomicRingBuf};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::nic::SparsePacket;
use crate::Address;

const IPV4_PROTO_TCP: u8 = 6;
const MAX_WINDOW_SIZE: u32 = 0x100000;	// 4MiB
const DEF_WINDOW_SIZE: u32 = 0x4000;	// 16KiB

pub fn init()
{
	::ipv4::register_handler(IPV4_PROTO_TCP, rx_handler_v4).unwrap();
}

#[path="tcp-lib/"]
/// Library types just for TCP
mod lib {
	pub mod rx_buffer;
}
use self::lib::rx_buffer::RxBuffer;

static CONNECTIONS: SharedMap<Quad, Mutex<Connection>> = SharedMap::new();
static PROTO_CONNECTIONS: SharedMap<Quad, ProtoConnection> = SharedMap::new();
static SERVERS: SharedMap<(Option<Address>,u16), Server> = SharedMap::new();

static S_PORTS: Mutex<PortPool> = Mutex::new(PortPool::new());

/// Find the local source address for the given remote address
// TODO: Shouldn't this get an interface handle instead?
fn get_outbound_ip_for(addr: &Address) -> Option<Address>
{
	match addr
	{
	Address::Ipv4(addr) => crate::ipv4::route_lookup(crate::ipv4::Address::zero(), *addr).map(|(laddr, _, _)| Address::Ipv4(laddr)),
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
		let packet_len = pre_header_reader.remain();
		// Pseudo header for checksum
		let sum_pseudo = match (src_addr,dest_addr)
			{
			(Address::Ipv4(s), Address::Ipv4(d)) =>
				::ipv4::calculate_checksum([
					// Big endian stores MSB first, so write the high word first
					(s.as_u32() >> 16) as u16, (s.as_u32() >> 0) as u16,
					(d.as_u32() >> 16) as u16, (d.as_u32() >> 0) as u16,
					IPV4_PROTO_TCP as u16, packet_len as u16,
					].iter().copied()),
			};
		let sum_header = hdr.checksum();
		let sum_options_and_data = {
			let mut pkt = pkt.clone();
			let psum_whole = !::ipv4::calculate_checksum( (0 .. (pre_header_reader.remain() - hdr_len) / 2).map(|_| pkt.read_u16n().unwrap()) );
			// Final byte is decoded as if there was a zero after it (so as 0x??00)
			let psum_partial = if pkt.remain() > 0 { (pkt.read_u8().unwrap() as u16) << 8} else { 0 };
			::ipv4::calculate_checksum([psum_whole, psum_partial].iter().copied())
			};
		let sum_total = ::ipv4::calculate_checksum([
			!sum_pseudo, !sum_header, !sum_options_and_data
			].iter().copied());
		if sum_total != 0 {
			log_error!("Incorrect checksum: 0x{:04x} != 0", sum_total);
		}
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
		c.lock().handle(&quad, &hdr, pkt);
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
				CONNECTIONS.insert(quad, Mutex::new(Connection::new_inbound(&hdr)));
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
	else if hdr.flags & !FLAG_ACK == FLAG_SYN
	{
		if let Some(s) = Option::or( SERVERS.get( &(Some(dest_addr), hdr.dest_port) ), SERVERS.get( &(None, hdr.dest_port) ) )
		{
			// Decrement the server's accept space
			if s.accept_space.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| if v == 0 { None } else { Some(v - 1) }).is_err() { 
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
			quad.send_packet(hdr.acknowledgement_number, hdr.sequence_number, FLAG_RST|(!hdr.flags & FLAG_ACK), 0, &[]);
		}
	}
	// Otherwise, drop
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
		Address::Ipv4(a) => crate::ipv4::send_packet(a, self.remote_addr.unwrap_ipv4(), IPV4_PROTO_TCP, hdr_pkt),
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
	state: ConnectionState,

	/// Sequence number of the next expected remote byte
	next_rx_seq: u32,
	/// Last ACKed sequence number
	last_rx_ack: u32,
	/// Received bytes
	rx_buffer: RxBuffer,
	/// Sequence number of the first byte in the RX buffer
	rx_buffer_seq: u32,

	rx_window_size_max: u32,
	rx_window_size: u32,

	/// Sequence number of last transmitted byte
	last_tx_seq: u32,
	/// Buffer of transmitted but not ACKed bytes
	tx_buffer: RingBuf<u8>,
	/// Offset of bytes actually sent (not just buffered)
	tx_bytes_sent: usize,
	/// Last received transmit window size
	tx_window_size: u32,
}
#[derive(Copy,Clone,Debug,PartialEq)]
enum ConnectionState
{
	//Closed,	// Unused

	SynSent,	// SYN sent by local, waiting for SYN-ACK
	//SynReceived,	// Server only, handled by PROTO_CONNECTIONS

	Established,

	FinWait1,	// FIN sent, waiting for reply (ACK or FIN)
	FinWait2,	// sent FIN acked, waiting for FIN from peer 
	Closing,	// Waiting for ACK of FIN (FIN sent and recieved)
	TimeWait,	// Waiting for timeout after local close

	ForceClose,	// RST recieved, waiting for user close
	CloseWait,	// FIN recieved, waiting for user to close (error set, wait for node close)
	LastAck,	// FIN sent and recieved, waiting for ACK

	Finished,
}
impl Connection
{
	/// Create a new connection from the ACK in a SYN-SYN,ACK-ACK
	fn new_inbound(hdr: &PktHeader) -> Self
	{
		Connection {
			state: ConnectionState::Established,
			next_rx_seq: hdr.sequence_number,
			last_rx_ack: hdr.sequence_number,
			rx_buffer_seq: hdr.sequence_number,
			rx_buffer: RxBuffer::new(2*DEF_WINDOW_SIZE as usize),

			rx_window_size_max: MAX_WINDOW_SIZE,	// Can be updated by the user
			rx_window_size: DEF_WINDOW_SIZE,

			last_tx_seq: hdr.acknowledgement_number,
			tx_buffer: RingBuf::new(2048),//hdr.window_size as usize),
			tx_bytes_sent: 0,
			tx_window_size: hdr.window_size as u32,
			}
	}

	fn new_outbound(quad: &Quad, sequence_number: u32) -> Self
	{
		log_trace!("Connection::new_outbound({:?}, {:#x})", quad, sequence_number);
		let mut rv = Connection {
			state: ConnectionState::SynSent,
			next_rx_seq: 0,
			last_rx_ack: 0,
			rx_buffer_seq: 0,
			rx_buffer: RxBuffer::new(2*DEF_WINDOW_SIZE as usize),

			rx_window_size_max: MAX_WINDOW_SIZE,	// Can be updated by the user
			rx_window_size: DEF_WINDOW_SIZE,

			last_tx_seq: sequence_number,
			tx_buffer: RingBuf::new(2048),
			tx_bytes_sent: 0,
			tx_window_size: 0,//hdr.window_size as u32,
			};
		rv.send_packet(quad, FLAG_SYN, &[]);
		rv
	}

	/// Handle inbound data
	fn handle(&mut self, quad: &Quad, hdr: &PktHeader, mut pkt: ::nic::PacketReader)
	{
		match self.state
		{
		//ConnectionState::Closed => return,
		ConnectionState::Finished => return,
		_ => {},
		}

		// Synchronisation request
		if hdr.flags & FLAG_SYN != 0 {
			// TODO: Send an ACK of the last recieved byte (should this be conditional?)
			if self.last_rx_ack != self.next_rx_seq {
			}
			//self.next_rx_seq = hdr.sequence_number;
		}
		// ACK of sent data
		if hdr.flags & FLAG_ACK != 0 {
			let in_flight = (self.last_tx_seq - hdr.acknowledgement_number) as usize;
			if in_flight > self.tx_buffer.len() {
				// TODO: Error, something funky has happened
			}
			else {
				let n_bytes = self.tx_buffer.len() - in_flight;
				log_debug!("{:?} ACQ {} bytes", quad, n_bytes);
				for _ in 0 .. n_bytes {
					self.tx_buffer.pop_front();
				}
			}
		}

		// Update the window size if it changes
		if self.tx_window_size != hdr.window_size as u32 {
			self.tx_window_size = hdr.window_size as u32;
		}
		
		let new_state = match self.state
		{
		//ConnectionState::Closed => return,

		// SYN sent by local, waiting for SYN-ACK
		ConnectionState::SynSent => {	
			if hdr.flags & FLAG_SYN != 0 {
				self.next_rx_seq += 1;
				if hdr.flags & FLAG_ACK != 0 {
					// Now established
					// TODO: Send ACK back
					self.send_ack(quad, "SYN-ACK");
					ConnectionState::Established
				}
				else {
					// Why did we get a plain SYN in this state?
					self.state
				}
			}
			else {
				// Ignore non-SYN
				self.state
			}
			},

		ConnectionState::Established =>
			if hdr.flags & FLAG_RST != 0 {
				// RST received, do an unclean close (reset by peer)
				// TODO: Signal to user that the connection is closing (error)
				ConnectionState::ForceClose
			}
			else if hdr.flags & FLAG_FIN != 0 {
				// FIN received, start a clean shutdown
				self.next_rx_seq += 1;
				// TODO: Signal to user that the connection is closing (EOF)
				ConnectionState::CloseWait
			}
			else {
				if pkt.remain() == 0 {
					// Pure ACK, no change
					if hdr.flags == FLAG_ACK {
						log_trace!("{:?} ACK only", quad);
					}
					else if self.next_rx_seq != hdr.sequence_number {
						log_trace!("{:?} Empty packet, unexpected seqeunce number {:x} != {:x}", quad, hdr.sequence_number, self.next_rx_seq);
					}
					else {
						// Counts as one byte
						self.next_rx_seq += 1;
						self.send_ack(quad, "Empty");
					}
				}
				else if hdr.sequence_number - self.next_rx_seq + pkt.remain() as u32 > MAX_WINDOW_SIZE {
					// Completely out of sequence
				}
				else {
					// In sequence.
					let mut start_ofs = (hdr.sequence_number - self.next_rx_seq) as i32;
					while start_ofs < 0 {
						pkt.read_u8().unwrap();
						start_ofs += 1;
					}
					let mut ofs = start_ofs as usize;
					while let Ok(b) = pkt.read_u8() {
						match self.rx_buffer.insert( (self.next_rx_seq - self.rx_buffer_seq) as usize + ofs, &[b])
						{
						Ok(_) => {},
						Err(e) => {
							log_error!("{:?} RX buffer push {:?}", quad, e);
							break;
							},
						}
						ofs += 1;
					}
					// Better idea: Have an ACQ point, and a window point. Buffer is double the window
					// Once the window point reaches 25% of the window from the ACK point
					if start_ofs == 0 {
						self.next_rx_seq += ofs as u32;
						// Calculate a maximum window size based on how much space is left in the buffer
						let buffered_len = self.next_rx_seq - self.rx_buffer_seq;	// How much data the user has buffered
						let cur_max_window = 2*self.rx_window_size_max - buffered_len;	// NOTE: 2* for some flex so the window can stay at max size
						if cur_max_window < self.rx_window_size {
							// Reduce the window size and send an ACQ (with the updated size)
							while cur_max_window < self.rx_window_size {
								self.rx_window_size /= 2;
							}
							self.send_ack(quad, "Constrain window");
						}
						else if self.next_rx_seq - self.last_rx_ack > self.rx_window_size/2 {
							// Send an ACK now, we've recieved a burst of data
							self.send_ack(quad, "Data burst");
						}
						else {
							// TODO: Schedule an ACK in a few hundred milliseconds
						}
					}

					if hdr.flags & FLAG_PSH != 0 {
						// TODO: Prod the user that there's new data?
					}
				}

				self.state
			},

		ConnectionState::CloseWait => {
			// Ignore all packets while waiting for the user to complete teardown
			self.state
			},
		ConnectionState::LastAck =>	// Waiting for ACK in FIN,FIN/ACK,ACK
			if hdr.flags & FLAG_ACK != 0 {
				ConnectionState::Finished
			}
			else {
				self.state
			},

		ConnectionState::FinWait1 =>	// FIN sent, waiting for reply (ACK or FIN)
			if hdr.flags & FLAG_FIN != 0 {
				// TODO: Check the sequence number vs the sequence for the FIN
				self.send_ack(quad, "SYN-ACK");
				ConnectionState::Closing
			}
			else if hdr.flags & FLAG_ACK != 0 {
				// TODO: Check the sequence number vs the sequence for the FIN
				ConnectionState::FinWait2
			}
			else {
				self.state
			},
		ConnectionState::FinWait2 =>
			if hdr.flags & FLAG_FIN != 0 {	// Got a FIN after the ACK, close
				ConnectionState::TimeWait
			}
			else {
				self.state
			},

		ConnectionState::Closing =>
			if hdr.flags & FLAG_ACK != 0 {
				// TODO: Check the sequence number vs the sequence for the FIN
				ConnectionState::TimeWait
			}
			else {
				self.state
			},

		ConnectionState::ForceClose => self.state,
		ConnectionState::TimeWait => self.state,

		ConnectionState::Finished => return,
		};

		self.state_update(quad, new_state);
	}

	fn state_update(&mut self, quad: &Quad, new_state: ConnectionState)
	{
		if self.state != new_state
		{
			log_trace!("{:?} {:?} -> {:?}", quad, self.state, new_state);
			self.state = new_state;

			// TODO: If transitioning to `Finished`, release the local port?
			// - Only for client connections.
			if let ConnectionState::Finished = self.state
			{
				release_port(&quad.local_addr, quad.local_port);
			}
		}
	}

	fn state_to_error(&self) -> Result<(), ConnError>
	{
		match self.state
		{
		ConnectionState::SynSent => {
			todo!("(quad=?) send/recv before established");
			},
		ConnectionState::Established => Ok( () ),
		ConnectionState::FinWait1
		| ConnectionState::FinWait2
		| ConnectionState::Closing
		| ConnectionState::TimeWait => Err( ConnError::LocalClosed ),

		ConnectionState::ForceClose => Err( ConnError::RemoteReset ),
		ConnectionState::CloseWait | ConnectionState::LastAck => Err( ConnError::RemoteClosed ),

		ConnectionState::Finished => Err( ConnError::LocalClosed ),
		}
	}
	fn send_data(&mut self, quad: &Quad, buf: &[u8]) -> Result<usize, ConnError>
	{
		// TODO: Is it valid to send before the connection is fully established?
		self.state_to_error()?;
		// 1. Determine how much data we can send (based on the TX window)
		let max_len = usize::saturating_sub(self.tx_window_size as usize, self.tx_buffer.len());
		let rv = ::core::cmp::min(buf.len(), max_len);
		// Add the data to the TX buffer
		for &b in &buf[..rv] {
			self.tx_buffer.push_back(b).expect("Incorrectly calculated `max_len` in tcp::Connection::send_data");
		}
		// If the buffer is full enough, do a send
		if self.tx_buffer.len() - self.tx_bytes_sent > 1400 /*|| self.first_tx_time.map(|t| now() - t > MAX_TX_DELAY).unwrap_or(false)*/
		{
			// Trigger a TX
			self.flush_send(quad);
		}
		else
		{
			// Kick a short timer, which will send data after it expires
			// - Keep kicking the timer as data flows through
			// - Have a maximum elapsed time with no packet sent.
			//if self.tx_timer.reset(MIN_TX_DELAY) == timer::ResetResult::WasStopped
			//{
			//	self.first_tx_time = Some(now());
			//}
		}
		todo!("{:?} send_data( min({}, {})={} )", quad, max_len, buf.len(), rv);
	}
	fn flush_send(&mut self, quad: &Quad)
	{
		loop
		{
			let nbytes = self.tx_buffer.len() - self.tx_bytes_sent;
			todo!("{:?} tx {}", quad, nbytes);
		}
		//self.first_tx_time = None;
	}
	fn recv_data(&mut self, _quad: &Quad, buf: &mut [u8]) -> Result<usize, ConnError>
	{
		self.state_to_error()?;
		//let valid_len = self.rx_buffer.valid_len();
		//let acked_len = u32::wrapping_sub(self.next_rx_seq, self.rx_buffer_seq);
		//let len = usize::min(valid_len, buf.len());
		Ok( self.rx_buffer.take(buf) )
	}

	fn send_packet(&mut self, quad: &Quad, flags: u8, data: &[u8])
	{
		log_debug!("{:?} send_packet({:02x} {}b)", quad, flags, data.len());
		quad.send_packet(self.last_tx_seq, self.next_rx_seq, flags, self.rx_window_size as u16, data);
	}
	fn send_ack(&mut self, quad: &Quad, msg: &str)
	{
		log_debug!("{:?} send_ack({:?})", quad, msg);
		// - TODO: Cancel any pending ACK
		// - Send a new ACK
		self.send_packet(quad, FLAG_ACK, &[]);
	}
	fn close(&mut self, quad: &Quad) -> Result<(), ConnError>
	{
		let new_state = match self.state
			{
			ConnectionState::SynSent => {
				todo!("{:?} close before established", quad);
				},
			ConnectionState::FinWait1
			| ConnectionState::FinWait2
			| ConnectionState::Closing
			| ConnectionState::TimeWait => return Err( ConnError::LocalClosed ),

			ConnectionState::LastAck => return Err( ConnError::RemoteClosed ),

			ConnectionState::Finished => return Err( ConnError::LocalClosed ),

			ConnectionState::CloseWait => {
				self.send_packet(quad, FLAG_FIN|FLAG_ACK, &[]);
				ConnectionState::LastAck
				},
			ConnectionState::ForceClose => {
				ConnectionState::Finished
				},
			ConnectionState::Established => {
				self.send_packet(quad, FLAG_FIN, &[]);
				ConnectionState::FinWait1
				},
			};
		self.state_update(quad, new_state);
		Ok( () )
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


pub struct ConnectionHandle(Quad);

#[derive(Debug)]
pub enum ConnError
{
	NoRoute,
	LocalClosed,
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
		CONNECTIONS.insert(quad, Mutex::new(conn));
		Ok( ConnectionHandle(quad) )
	}
	pub fn send_data(&self, buf: &[u8]) -> Result<usize, ConnError>
	{
		match CONNECTIONS.get(&self.0)
		{
		None => panic!("Connection {:?} removed before handle dropped", self.0),
		Some(v) => v.lock().send_data(&self.0, buf),
		}
	}

	pub fn recv_data(&self, buf: &mut [u8]) -> Result<usize, ConnError>
	{
		match CONNECTIONS.get(&self.0)
		{
		None => panic!("Connection {:?} removed before handle dropped", self.0),
		Some(v) => v.lock().recv_data(&self.0, buf),
		}
	}

	pub fn close(&mut self) -> Result<(), ConnError>
	{
		match CONNECTIONS.get(&self.0)
		{
		None => panic!("Connection {:?} removed before handle dropped", self.0),
		Some(v) => v.lock().close(&self.0),
		}
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

