//! TCP connection logic

use ::kernel::lib::ring_buffer::RingBuf;
use super::lib::rx_buffer::RxBuffer;
use super::{Quad,WORKER_CV};
use super::ConnError;
use super::{FLAG_SYN,FLAG_ACK,FLAG_PSH,FLAG_RST,FLAG_FIN};

const DEF_TX_WINDOW_SIZE: u32 = 0x1000;
const DEF_RX_WINDOW_SIZE: u32 = 0x4000;	// 16KiB
const MAX_WINDOW_SIZE: u32 = 0x100000;	// 4MiB
/// Base timeout between attempting to send a packet and the first retransmit attempt
const RETRANSMIT_TIMEOUT_MS: usize = 200;
/// Maximum segment size (i.e. the largest amount of data in a single IP frame)
const MSS: usize = 1400;

pub struct Connection
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

	tx_state: ConnectionTxState,
}
struct ConnectionTxState {
	/// Buffer of outbound bytes (data pending an incoming ACK)
	buffer: RingBuf<u8>,
	/// Sequence number of the next byte to be sent
	next_tx_seq: u32,
	
	/// Number of bytes that have been sent, but not ACKed
	sent_bytes: usize,

	/// Last received TX window size
	max_tx_window_size: u32,
	/// Current TX window size (can be reduced if packet loss is seen)
	cur_tx_window_size: u32,

	// TODO: Pending flags?

	// -- Timers and state for transmit
	/// Timer use to ensure that we get ACKs in a suitable time.
	retransmit_timer: ::kernel::time::Timer,
	/// Flag that forces a TX on the next opportunity (e.g. the buffer has a packet worth of data, or a flush was requested)
	force_tx: bool,
	/// Send an ACK in the next opportunity
	pending_ack: bool,
}
impl ConnectionTxState {
	fn new(tx_seq: u32, init_window_size: u32) -> Self {
		ConnectionTxState {
			buffer: RingBuf::new(DEF_TX_WINDOW_SIZE as usize),
			next_tx_seq: tx_seq,

			sent_bytes: 0,
			max_tx_window_size: init_window_size,
			cur_tx_window_size: init_window_size,
			retransmit_timer: ::kernel::time::Timer::new(),
			force_tx: false,
			pending_ack: false,
		}
	}
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
	pub(super) fn new_inbound(hdr: &super::PktHeader) -> Self
	{
		Connection {
			state: ConnectionState::Established,
			next_rx_seq: hdr.sequence_number,
			last_rx_ack: hdr.sequence_number,
			rx_buffer_seq: hdr.sequence_number,
			rx_buffer: RxBuffer::new(2*DEF_RX_WINDOW_SIZE as usize),

			rx_window_size_max: MAX_WINDOW_SIZE,	// Can be updated by the user
			rx_window_size: DEF_RX_WINDOW_SIZE,

			tx_state: ConnectionTxState::new(hdr.acknowledgement_number, hdr.window_size as u32),
			}
	}

	pub(super) fn new_outbound(quad: &Quad, sequence_number: u32) -> Self
	{
		log_trace!("Connection::new_outbound({:?}, {:#x})", quad, sequence_number);
		let mut rv = Connection {
			state: ConnectionState::SynSent,
			next_rx_seq: 0,
			last_rx_ack: 0,
			rx_buffer_seq: 0,
			rx_buffer: RxBuffer::new(2*DEF_RX_WINDOW_SIZE as usize),

			rx_window_size_max: MAX_WINDOW_SIZE,	// Can be updated by the user
			rx_window_size: DEF_RX_WINDOW_SIZE,

			tx_state: ConnectionTxState::new(sequence_number, DEF_TX_WINDOW_SIZE),
			};
		rv.send_empty_packet(quad, FLAG_SYN);
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
			// TODO: Send an ACK of the last recieved byte (should this be conditional?)
			if self.last_rx_ack != self.next_rx_seq {
			}
			//self.next_rx_seq = hdr.sequence_number;
		}
		// ACK of sent data
		if hdr.flags & FLAG_ACK != 0 {
			let in_flight = self.tx_state.next_tx_seq.wrapping_sub(1).wrapping_sub(hdr.acknowledgement_number) as usize;
			if in_flight > self.tx_state.buffer.len() {
				// TODO: Error, something funky has happened
			}
			else {
				let n_bytes = self.tx_state.buffer.len() - in_flight;
				log_debug!("{:?} ACQ {} bytes", quad, n_bytes);
				for _ in 0 .. n_bytes {
					self.tx_state.buffer.pop_front();
					self.tx_state.sent_bytes -= 1;
				}
				// If there are no un-acked bytes, and there's pending bytes. Trigger a re-send
				if self.tx_state.sent_bytes == 0 && self.tx_state.buffer.len() > 0 {
					self.tx_state.force_tx = true;
					super::WORKER_CV.wake_one();
				}
				else {
					// Since we've seen an ACK, reset the retransmit time
					if self.tx_state.sent_bytes == 0 {
						self.tx_state.retransmit_timer.clear();
					}
					else {
						// TODO: Maintain a retransmit timer and double it each time we need to retransmit
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
							// - Just set a flag so the next outbound packet ACKs
							self.tx_state.pending_ack = true;
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
	/// Enqueue data to be sent
	pub(super) fn send_data(&mut self, _quad: &Quad, buf: &[u8]) -> Result<usize, ConnError>
	{
		// TODO: Is it valid to send before the connection is fully established?
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
			WORKER_CV.wake_one();
			//self.flush_send(quad);
		}
		else
		{
			// Just enqueue the data, the RX logic will trigger a re-send on ACK
			log_trace!("{:?} waiting for nagle", _quad);
		}
		Ok(rv)
	}
	/// Pull data from the received buffer
	pub(super) fn recv_data(&mut self, _quad: &Quad, buf: &mut [u8]) -> Result<usize, ConnError>
	{
		self.state_to_error()?;
		//let valid_len = self.rx_buffer.valid_len();
		//let acked_len = u32::wrapping_sub(self.next_rx_seq, self.rx_buffer_seq);
		//let len = usize::min(valid_len, buf.len());
		Ok( self.rx_buffer.take(buf) )
	}

	/// Run TX tasks (from the TX worker)
	pub(super) fn run_tasks(&mut self, quad: &Quad) -> Option<::kernel::time::TickCount>
	{
		use ::kernel::futures::block_on;

		let flags = {
			let mut flags = 0u8;
			if ::core::mem::replace(&mut self.tx_state.pending_ack, false) {
				flags |= FLAG_ACK;
			}
			// TODO: Use a better method of picking when to PSH
			if false && self.tx_state.force_tx {
				flags |= FLAG_PSH;
			}
			flags
			};

		if self.tx_state.retransmit_timer.is_expired() {
			// Re-send any pending data (and reduce our TX window size?)
			let len = self.tx_state.buffer.len().min(MSS);
			log_trace!("{:?} Retransmit {:#x} {} bytes", quad, flags, len);
			let data = self.tx_state.buffer.get_slices(0..len);
			// `next_tx_seq` is the sequence number of the next new byte to be sent
			// - I.e. the byte at `buffer[sent_bytes]`
			// - So, we want to subtract the number of bytes between `data.len()` and `sent_bytes`
			let seq_ofs = (self.tx_state.sent_bytes as u32).wrapping_sub(len as u32);
			let seq = self.tx_state.next_tx_seq.wrapping_sub(seq_ofs);
			block_on(quad.send_packet(seq, self.next_rx_seq, flags, self.rx_window_size as u16, data.0, data.1));
			// TODO: Double this timer each time we need to resend (and halve it on successful reception)
			self.tx_state.retransmit_timer.reset(RETRANSMIT_TIMEOUT_MS as u64);
		}
		else if ::core::mem::replace(&mut self.tx_state.force_tx, false) {
			// Send the new data
			let nbytes = self.tx_state.buffer.len() - self.tx_state.sent_bytes;
			let nbytes = nbytes.min(MSS);
			let data = self.tx_state.buffer.get_slices(self.tx_state.sent_bytes .. self.tx_state.sent_bytes + nbytes);
			let seq = self.tx_state.next_tx_seq;
			log_trace!("{:?} TX forced {:#x} {} bytes", quad, flags, nbytes);
			block_on(quad.send_packet(seq, self.next_rx_seq, flags, self.rx_window_size as u16, data.0, data.1));
			// TODO: Some flags act as a pseudo-byte if in an empty packet
			self.tx_state.next_tx_seq = self.tx_state.next_tx_seq.wrapping_add( nbytes as u32 );
		}
		else {
			// Nothing to do.
		}

		let mut rv = None;
		super::earliest_timestamp(&mut rv, self.tx_state.retransmit_timer.get_expiry());
		rv
	}

	fn send_empty_packet(&mut self, quad: &Quad, flags: u8)
	{
		log_debug!("{:?} send_packet({:02x})", quad, flags);
		// TODO: Enqueue instead of blocking?
		::kernel::futures::block_on(quad.send_packet(self.tx_state.next_tx_seq, self.next_rx_seq, flags, self.rx_window_size as u16, &[], &[]));
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
				self.send_empty_packet(quad, FLAG_FIN|FLAG_ACK);
				ConnectionState::LastAck
				},
			ConnectionState::ForceClose => {
				ConnectionState::Finished
				},
			ConnectionState::Established => {
				self.send_empty_packet(quad, FLAG_FIN);
				ConnectionState::FinWait1
				},
			};
		self.state_update(quad, new_state);
		Ok( () )
	}
}