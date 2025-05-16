//! TCP connection logic

use ::kernel::lib::ring_buffer::RingBuf;
use super::rx_buffer::RxBuffer;
use super::{Quad,WORKER_CV};
use super::ConnError;
use super::{FLAG_SYN,FLAG_ACK,FLAG_PSH,FLAG_RST,FLAG_FIN};

const DEF_TX_WINDOW_SIZE: u32 = 0x1000;
const DEF_RX_WINDOW_SIZE: u32 = 0x4000;	// 16KiB
const MAX_WINDOW_SIZE: u32 = 0x100000;	// 4MiB
/// Base timeout between attempting to send a packet and the first retransmit attempt
const RETRANSMIT_TIMEOUT_MS: usize = 200;
/// Maximum number of attempt to re-transmit a packet before closing the connection
const MAX_CONN_ATTEMPTS: u32 = 5;
/// Maximum segment size (i.e. the largest amount of data in a single IP frame)
const MSS: usize = 1400;

pub struct Connection
{
	state: ConnectionState,
	conn_waiters: ::kernel::threads::SleepObjectSet,

	/// Sequence number of the next expected remote byte
	next_rx_seq: SeqNum,
	/// Last ACKed sequence number
	last_rx_ack: SeqNum,
	/// Received bytes
	rx_buffer: RxBuffer,
	rx_waiters: ::kernel::threads::SleepObjectSet,
	/// Sequence number of the first byte in the RX buffer
	rx_buffer_seq: SeqNum,

	rx_window_size_max: u32,
	rx_window_size: u32,

	tx_state: ConnectionTxState,
	tx_waiters: ::kernel::threads::SleepObjectSet,
}

#[derive(Copy,Clone,Debug,PartialEq)]
struct SeqNum(u32);
impl ::core::fmt::Display for SeqNum {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:#x}", self.0)
	}
}
impl ::core::ops::Add for SeqNum {
	type Output = Self;
	fn add(self, rhs: Self) -> Self::Output {
		SeqNum(self.0 + rhs.0)
	}
}
impl ::core::ops::Sub for SeqNum {
	type Output = Self;
	fn sub(self, rhs: Self) -> Self::Output {
		SeqNum(self.0 - rhs.0)
	}
}

struct ConnectionTxState {
	/// Buffer of outbound bytes (data pending an incoming ACK)
	buffer: RingBuf<u8>,
	/// Sequence number of the next byte to be sent
	next_tx_seq: SeqNum,
	
	/// Number of bytes that have been sent, but not ACKed
	/// 
	/// Could be less than `buffer.len()` as this is only incremented on send
	sent_bytes: usize,

	/// Last received TX window size
	max_tx_window_size: u32,
	/// Current TX window size (can be reduced if packet loss is seen)
	cur_tx_window_size: u32,

	// TODO: Pending flags?

	// -- Timers and state for transmit
	/// Number of re-transmit attempts since the last successful receive
	retransmit_attempts: u32,
	/// Timer used to ensure that we get ACKs in a suitable time.
	retransmit_timer: ::kernel::time::Timer,
	/// Timer to collate multiple sends
	nagle_timer: ::kernel::time::Timer,
	/// Flag that forces a TX on the next opportunity (e.g. the buffer has a packet worth of data, or a flush was requested)
	force_tx: bool,
	/// Send an ACK in the next opportunity
	pending_ack: bool,
}
impl ConnectionTxState {
	fn new(tx_seq: u32, init_window_size: u32) -> Self {
		ConnectionTxState {
			buffer: RingBuf::new(DEF_TX_WINDOW_SIZE as usize),
			next_tx_seq: SeqNum(tx_seq),

			sent_bytes: 0,
			max_tx_window_size: init_window_size,
			cur_tx_window_size: init_window_size,
			retransmit_attempts: 0,
			retransmit_timer: ::kernel::time::Timer::new(),
			nagle_timer: ::kernel::time::Timer::new(),
			force_tx: false,
			pending_ack: false,
		}
	}
}
#[derive(Copy,Clone,Debug,PartialEq)]
enum ConnectionState
{
	//Closed,	// Unused

	/// SYN sent by local, waiting for SYN-ACK
	SynSent,
	/// (NON-RFC) Indicates that re-transmit attempts have been exhausted, and the connection has failed
	Timeout,
	//SynReceived,	// Server only, handled by PROTO_CONNECTIONS

	Established,

	FinWait1,	// FIN sent, waiting for reply (ACK or FIN)
	FinWait2,	// sent FIN acked, waiting for FIN from peer 
	Closing,	// Waiting for ACK of FIN (FIN sent and received)
	TimeWait,	// Waiting for timeout after local close

	ForceClose,	// RST received, waiting for user close
	CloseWait,	// FIN received, waiting for user to close (error set, wait for node close)
	LastAck,	// FIN sent and received, waiting for ACK

	Finished,
}
impl Connection
{
	/// Create a new connection from the ACK in a SYN-SYN,ACK-ACK
	pub(super) fn new_inbound(hdr: &super::PktHeader) -> Self
	{
		Connection {
			state: ConnectionState::Established,
			conn_waiters: Default::default(),
			next_rx_seq: SeqNum(hdr.sequence_number),
			last_rx_ack: SeqNum(hdr.sequence_number),
			rx_buffer_seq: SeqNum(hdr.sequence_number),
			rx_buffer: RxBuffer::new(2*DEF_RX_WINDOW_SIZE as usize),
			rx_waiters: Default::default(),

			rx_window_size_max: MAX_WINDOW_SIZE,	// Can be updated by the user
			rx_window_size: DEF_RX_WINDOW_SIZE,

			tx_state: ConnectionTxState::new(hdr.acknowledgement_number, hdr.window_size as u32),
			tx_waiters: Default::default(),
			}
	}

	pub(super) fn new_outbound(quad: &Quad, sequence_number: u32) -> Self
	{
		log_trace!("Connection::new_outbound({:?}, {:#x})", quad, sequence_number);
		let mut rv = Connection {
			state: ConnectionState::SynSent,
			conn_waiters: Default::default(),
			next_rx_seq: SeqNum(0),
			last_rx_ack: SeqNum(0),
			rx_buffer_seq: SeqNum(0),
			rx_buffer: RxBuffer::new(2*DEF_RX_WINDOW_SIZE as usize),
			rx_waiters: Default::default(),

			rx_window_size_max: MAX_WINDOW_SIZE,	// Can be updated by the user
			rx_window_size: DEF_RX_WINDOW_SIZE,

			tx_state: ConnectionTxState::new(sequence_number, DEF_TX_WINDOW_SIZE),
			tx_waiters: Default::default(),
			};
		rv.send_empty_packet(quad, FLAG_SYN);
		// TODO: This should be a little more formalised. A SYN should count as data, and be resent the same way
		rv.tx_state.next_tx_seq = rv.tx_state.next_tx_seq + SeqNum(1);
		rv.tx_state.retransmit_timer.reset(RETRANSMIT_TIMEOUT_MS as u64);
		rv
	}

	/// Handle an inbound packet
	pub(super) fn handle(&mut self, quad: &Quad, hdr: &super::PktHeader, mut pkt: crate::nic::PacketReader)
	{
		match self.state
		{
		//ConnectionState::Closed => return,
		ConnectionState::Finished => return,
		_ => {},
		}

		// Synchronisation request
		if hdr.flags & FLAG_SYN != 0 {
			// TODO: SYN counts as a seqence increment?

			// TODO: Send an ACK of the last received byte (should this be conditional?)
			if self.last_rx_ack != self.next_rx_seq {
			}
			//self.next_rx_seq = hdr.sequence_number;
		}
		// ACK of sent data
		if hdr.flags & FLAG_ACK != 0 {
			// TODO: There are some pseudo-bytes (a SYN flag counts as a transmitted byte)
			log_trace!("{:?} ACK {} vs {}", quad, SeqNum(hdr.acknowledgement_number), self.tx_state.next_tx_seq);
			// Determine how many bytes are NOT acked by this packet
			// - Which can be used to determine how many are ACKed
			let in_flight = (self.tx_state.next_tx_seq - SeqNum(hdr.acknowledgement_number)).0 as usize;
			if in_flight > self.tx_state.sent_bytes {
				// TODO: Error, something funky has happened
				log_error!("Oops? in_flight={} > sent_bytes={}", in_flight, self.tx_state.sent_bytes);
			}
			else {
				assert!(self.tx_state.sent_bytes <= self.tx_state.buffer.len());
				let n_bytes = self.tx_state.sent_bytes - in_flight;
				log_debug!("{:?} ACK {} bytes", quad, n_bytes);
				for _ in 0 .. n_bytes {
					self.tx_state.buffer.pop_front();
					self.tx_state.sent_bytes -= 1;
				}
				// If any bytes were acked, then clear retransmit attempt count
				if n_bytes > 0 {
					self.tx_state.retransmit_attempts = 0;
				}
				// If there are no un-acked bytes, and there's pending bytes. Trigger a re-send
				if self.tx_state.sent_bytes == 0 && self.tx_state.buffer.len() > 0 {
					self.tx_state.force_tx = true;
					WORKER_CV.wake_one();
				}
				else {
					// Since we've seen an ACK, reset the retransmit time
					if self.tx_state.sent_bytes == 0 {
						self.tx_state.retransmit_timer.clear();
					}
					else {
						// TODO: Maintain a retransmit timer and double it each time we need to retransmit
						// TODO: Track estimated latency and set this based on the RTT
						self.tx_state.retransmit_timer.reset(RETRANSMIT_TIMEOUT_MS as u64);
					}
				}
			}
		}

		// Update the window size if it changes
		if self.tx_state.max_tx_window_size != hdr.window_size as u32 {
			log_debug!("{:?} Max TX window changed: {} -> {}", quad, self.tx_state.max_tx_window_size, hdr.window_size);
			self.tx_state.max_tx_window_size = hdr.window_size as u32;
		}
		// If there were acked bytes, then signal that TX is now possible again
		if self.tx_state.buffer.len() < self.rx_window_size as usize {
			self.tx_waiters.signal();
		}
		
		let new_state = match self.state
		{
		//ConnectionState::Closed => return,

		// SYN sent by local, waiting for SYN-ACK
		ConnectionState::SynSent => {	
			if hdr.flags & FLAG_SYN != 0 {
				self.next_rx_seq = SeqNum(hdr.sequence_number) + SeqNum(1);
				self.rx_buffer_seq = self.next_rx_seq;
				if hdr.flags & FLAG_ACK != 0 {
					// Now established
					// - Send ACK back
					self.send_ack(quad, "SYN-ACK");
					self.tx_waiters.signal();
					self.tx_state.retransmit_timer.clear();
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
				self.next_rx_seq = self.next_rx_seq + SeqNum(1);
				// TODO: Signal to user that the connection is closing (EOF)
				ConnectionState::CloseWait
			}
			else {
				if pkt.remain() == 0 {
					// Pure ACK, no change
					if hdr.flags == FLAG_ACK {
						log_trace!("{:?} ACK only", quad);
					}
					else if self.next_rx_seq != SeqNum(hdr.sequence_number) {
						log_trace!("{:?} Empty packet, unexpected sequence number {} != {}", quad, SeqNum(hdr.sequence_number), self.next_rx_seq);
					}
					else {
						// Counts as one byte
						self.next_rx_seq = self.next_rx_seq + SeqNum(1);
						self.send_ack(quad, "Empty");
					}
				}
				else if (SeqNum(hdr.sequence_number) - self.next_rx_seq + SeqNum(pkt.remain() as u32)).0 > MAX_WINDOW_SIZE {
					// Completely out of sequence
				}
				else {
					// In sequence.
					let mut start_ofs = (SeqNum(hdr.sequence_number) - self.next_rx_seq).0 as i32;
					while start_ofs < 0 {
						pkt.read_u8().unwrap();
						start_ofs += 1;
					}
					let mut ofs = start_ofs as usize;
					log_debug!("{:?} RX: buf_ofs={}+{}", quad, self.next_rx_seq - self.rx_buffer_seq, ofs);
					while let Ok(b) = pkt.read_u8() {
						//let ofs_0 = self.next_rx_seq.wrapping_sub(self.rx_buffer_seq);
						//let ofs_0 = if ofs_0 > MAX_CONN_ATTEMPTS
						match self.rx_buffer.insert( (self.next_rx_seq - self.rx_buffer_seq).0 as usize + ofs, &[b])
						{
						Ok(_) => {},
						Err(e) => {
							log_error!("{:?} RX buffer push {:?}", quad, e);
							break;
							},
						}
						ofs += 1;
					}
					log_debug!("{:?} RX: start_ofs={}, ofs={}", quad, start_ofs, ofs);
					// Better idea: Have an ACQ point, and a window point. Buffer is double the window
					// Once the window point reaches 25% of the window from the ACK point
					if start_ofs == 0 {
						self.next_rx_seq = self.next_rx_seq + SeqNum(ofs as u32);

						// Calculate a maximum window size based on how much space is left in the buffer
						let buffered_len = (self.next_rx_seq - self.rx_buffer_seq).0;	// How much data the user has buffered
						let cur_max_window = 2*self.rx_window_size_max - buffered_len;	// NOTE: 2* for some flex so the window can stay at max size
						if cur_max_window < self.rx_window_size {
							// Reduce the window size and send an ACQ (with the updated size)
							while cur_max_window < self.rx_window_size {
								self.rx_window_size /= 2;
							}
							self.send_ack(quad, "Constrain window");
						}
						else if (self.next_rx_seq - self.last_rx_ack).0 > self.rx_window_size/2 {
							// Send an ACK now, we've received a burst of data
							self.send_ack(quad, "Data burst");
						}
						else {
							// TODO: Schedule an ACK in a few hundred milliseconds
							// - Just set a flag so the next outbound packet ACKs
							self.tx_state.pending_ack = true;
						}
					}

					if self.rx_buffer.valid_len() > 0 {
						self.rx_waiters.signal();
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
		ConnectionState::Timeout => {
			log_trace!("{:?} Packet received after timeout declared", quad);
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
				self.send_ack(quad, "FIN-ACK");
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
				super::release_port(&quad.local_addr, quad.local_port);
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
		ConnectionState::Timeout => Err( ConnError::TimedOut ),
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

	/// Indicates that the socket is (or was) connected
	pub(super) fn connection_complete(&self) -> bool {
		match self.state
		{
		ConnectionState::SynSent => false,
		_ => true,
		}
	}
	pub(super) fn connected_wait_bind(&mut self, obj: &::kernel::threads::SleepObject) {
		self.conn_waiters.add(obj);
		if self.connection_complete() {
			obj.signal();
		}
	}
	pub(super) fn connected_wait_unbind(&mut self, obj: &::kernel::threads::SleepObject) -> bool {
		self.conn_waiters.remove(obj);
		self.connection_complete()
	}

	/// Enqueue data to be sent
	pub(super) fn send_data(&mut self, _quad: &Quad, buf: &[u8]) -> Result<usize, ConnError>
	{
		// TODO: Is it valid to send before the connection is fully established?
		// - Should this block until then?
		if let ConnectionState::SynSent = self.state {
			return Ok(0);
		}
		self.state_to_error()?;
		// 1. Determine how much data we can send (based on the TX window)
		let max_len = usize::saturating_sub(self.tx_state.cur_tx_window_size as usize, self.tx_state.buffer.len());
		let rv = ::core::cmp::min(buf.len(), max_len);
		log_debug!("{:?} send_data({}/{})", _quad, rv, buf.len());
		// Add the data to the TX buffer
		for &b in &buf[..rv] {
			self.tx_state.buffer.push_back(b).expect("Incorrectly calculated `max_len` in tcp::Connection::send_data");
		}
		
		// Nagle algorithm!
		// Only send if:
		// - There's no unsent data in the buffer, OR
		// - There's more than 1MSS unsent in the buffer
		if self.tx_state.sent_bytes == 0 || self.tx_state.buffer.len() - self.tx_state.sent_bytes >= MSS
		{
			log_trace!("{:?} forcing a send", _quad);
			// Force a TX
			self.tx_state.force_tx = true;
		}
		else
		{
			// Just enqueue the data, the RX logic will trigger a re-send on ACK
			log_trace!("{:?} waiting for nagle", _quad);
			if self.tx_state.nagle_timer.get_expiry().is_none() {
				self.tx_state.nagle_timer.reset(100);
			}
		}
		WORKER_CV.wake_one();
		Ok(rv)
	}
	pub(super) fn send_ready(&self) -> bool {
		match self.state
		{
		ConnectionState::Established => {
			let max_len = usize::saturating_sub(self.tx_state.cur_tx_window_size as usize, self.tx_state.buffer.len());
			max_len > 0
		},
		_ => false,
		}
	}
	pub(super) fn send_wait_bind(&mut self, obj: &::kernel::threads::SleepObject) {
		self.tx_waiters.add(obj);
		if self.send_ready() {
			obj.signal();
		}
	}
	pub(super) fn send_wait_unbind(&mut self, obj: &::kernel::threads::SleepObject) -> bool {
		self.tx_waiters.remove(obj);
		self.send_ready()
	}

	/// Pull data from the received buffer
	pub(super) fn recv_data(&mut self, _quad: &Quad, buf: &mut [u8]) -> Result<usize, ConnError>
	{
		if let ConnectionState::SynSent = self.state {
			return Ok(0);
		}
		self.state_to_error()?;
		let rv = self.rx_buffer.take(buf);
		self.rx_buffer_seq = self.rx_buffer_seq + SeqNum(rv as u32);
		Ok( rv )
	}
	pub(super) fn recv_ready(&mut self) -> bool {
		self.rx_buffer.valid_len() > 0
	}
	pub(super) fn recv_wait_bind(&mut self, obj: &::kernel::threads::SleepObject) {
		self.rx_waiters.add(obj);
		if self.recv_ready() {
			obj.signal();
		}
	}
	pub(super) fn recv_wait_unbind(&mut self, obj: &::kernel::threads::SleepObject) -> bool {
		self.rx_waiters.remove(obj);
		self.recv_ready()
	}

	/// Run TX tasks (from the TX worker)
	pub(super) fn run_tasks(&mut self, quad: &Quad) -> Option<::kernel::time::TickCount>
	{
		use ::kernel::futures::block_on;

		let flags = {
			let mut flags = 0;
			// NOTE: slirp ignores any packet that isn't an ACK :(
			if ::core::mem::replace(&mut self.tx_state.pending_ack, false) || true {
				flags |= FLAG_ACK;
			}
			// TODO: Use a better method of picking when to PSH
			// - Want to set PSH when sending the last bytes of a usermode `send` call
			// - However, that may be after several packets have been sent (if it was a big write)
			// - Could have a variable for `push_at` that is set to the sequence number for the next PSH
			//   - Should this support a queue? OR, should it be updated on every `send` call?
			if self.tx_state.buffer.len() - self.tx_state.sent_bytes > 0 && self.tx_state.force_tx {
				flags |= FLAG_PSH;
			}
			flags
			};

		if self.tx_state.retransmit_timer.is_expired() {
			self.tx_state.retransmit_timer.clear();
			self.tx_state.retransmit_attempts += 1;
			match self.state {
			ConnectionState::SynSent => {
				if self.tx_state.retransmit_attempts == MAX_CONN_ATTEMPTS {
					log_trace!("{:?} Connection timeout: SynSent", quad);
					self.state = ConnectionState::Timeout;
					self.tx_waiters.signal();
					self.rx_waiters.signal();
					self.conn_waiters.signal();
				}
				else {
					// Re-send the SYN
					log_trace!("{:?} Retransmit initial SYN", quad);
					self.send_empty_packet(quad, FLAG_SYN);
					self.tx_state.retransmit_timer.reset(RETRANSMIT_TIMEOUT_MS as u64 * (self.tx_state.retransmit_attempts + 1) as u64);
				}
				},
			ConnectionState::Established => {
				if self.tx_state.retransmit_attempts == MAX_CONN_ATTEMPTS {
					log_trace!("{:?} Connection timeout: Established", quad);
					self.state = ConnectionState::Timeout;
					self.tx_waiters.signal();
					self.rx_waiters.signal();
					self.conn_waiters.signal();
				}
				else {
					// Re-send any pending data (and reduce our TX window size?)
					let len = self.tx_state.buffer.len().min(MSS);
					//assert!(self.tx_state.sent_bytes >= len);	// This could fail, if data has been queued but not sent
					let len = len.min(self.tx_state.sent_bytes);
					if len > 0 {
						log_trace!("{:?} Retransmit {:#x} {} bytes", quad, flags, len);
						let data = self.tx_state.buffer.get_slices(0..len);
						// `next_tx_seq` is the sequence number of the next new byte to be sent
						// - I.e. the byte at `buffer[sent_bytes]`
						// - So, we want to subtract the number of bytes between `data.len()` and `sent_bytes`
						let seq = self.tx_state.next_tx_seq - SeqNum(self.tx_state.sent_bytes as u32);
						block_on(quad.send_packet(seq.0, self.next_rx_seq.0, flags, self.rx_window_size as u16, data.0, data.1));
						// TODO: Double this timer each time we need to resend (and halve it on successful reception)
						self.tx_state.retransmit_timer.reset(RETRANSMIT_TIMEOUT_MS as u64 * (self.tx_state.retransmit_attempts + 1) as u64);
					}
				}
				},
			_ => {},
			}
		}
		else if ::core::mem::replace(&mut self.tx_state.force_tx, false) || self.tx_state.nagle_timer.is_expired() {
			// Send the new data
			let nbytes = self.tx_state.buffer.len() - self.tx_state.sent_bytes;
			let nbytes = nbytes.min(MSS);
			let data = self.tx_state.buffer.get_slices(self.tx_state.sent_bytes .. self.tx_state.sent_bytes + nbytes);
			let seq = self.tx_state.next_tx_seq;
			log_trace!("{:?} TX {} {:#x} {} bytes", quad, if self.tx_state.nagle_timer.is_expired() { "ready" } else { "forced" }, flags, nbytes);
			block_on(quad.send_packet(seq.0, self.next_rx_seq.0, flags, self.rx_window_size as u16, data.0, data.1));
			// TODO: Some flags act as a pseudo-byte if in an empty packet
			self.tx_state.next_tx_seq = self.tx_state.next_tx_seq + SeqNum( nbytes as u32 );
			self.tx_state.sent_bytes += nbytes;
			self.tx_state.nagle_timer.clear();
			// If the retransmit timer is stopped, start it again
			if self.tx_state.retransmit_timer.get_expiry().is_none() {
				self.tx_state.retransmit_timer.reset(RETRANSMIT_TIMEOUT_MS as u64);
			}
		}
		else {
			// Nothing to do.
		}

		let mut rv = None;
		super::earliest_timestamp(&mut rv, self.tx_state.retransmit_timer.get_expiry());
		super::earliest_timestamp(&mut rv, self.tx_state.nagle_timer.get_expiry());
		rv
	}

	fn send_empty_packet(&mut self, quad: &Quad, flags: u8)
	{
		log_debug!("{:?} send_packet({:02x})", quad, flags);
		// TODO: Enqueue instead of blocking?
		::kernel::futures::block_on(quad.send_packet(self.tx_state.next_tx_seq.0, self.next_rx_seq.0, flags, self.rx_window_size as u16, &[], &[]));
	}
	fn send_ack(&mut self, quad: &Quad, msg: &str)
	{
		log_debug!("{:?} send_ack({:?})", quad, msg);
		// - TODO: Cancel any pending ACK
		// - Send a new ACK
		self.tx_state.pending_ack = true;
		self.tx_state.force_tx = true;
		WORKER_CV.wake_one();
	}

	/// User requests the connection be closed
	pub(super) fn close(&mut self, quad: &Quad) -> Result<(), ConnError>
	{
		let new_state = match self.state
			{
			ConnectionState::FinWait1
			| ConnectionState::FinWait2
			| ConnectionState::Closing
			| ConnectionState::TimeWait => return Err( ConnError::LocalClosed ),

			ConnectionState::LastAck => return Err( ConnError::RemoteClosed ),

			ConnectionState::Finished => return Err( ConnError::LocalClosed ),

			ConnectionState::CloseWait => {
				self.send_empty_packet(quad, FLAG_FIN|FLAG_ACK);
				ConnectionState::LastAck
				},
			ConnectionState::ForceClose => {
				ConnectionState::Finished
				},
			ConnectionState::SynSent
			| ConnectionState::Timeout
			| ConnectionState::Established => {
				self.send_empty_packet(quad, FLAG_FIN);
				ConnectionState::FinWait1
				},
			};
		self.state_update(quad, new_state);
		Ok( () )
	}
}