//! ICMPv6
use kernel::lib::ring_buffer::RingBuf;
use kernel::lib::Vec;
use kernel::lib::VecMap;
use kernel::sync::{mutex::LazyMutexDefault,RwLock,Mutex};

use super::Address;

pub const NEXT_HEADER: u8 = 58;

static WORKER_SLEEP: ::kernel::sync::EventChannel = ::kernel::sync::EventChannel::new();
static PENDING_PACKETS: LazyMutexDefault<PendingPackets> = LazyMutexDefault::new();
static RUNNING_PINGS: RwLock<VecMap<(Address, u16), Mutex<PingState>>> = RwLock::new(VecMap::new());

struct PendingPackets {
	_worker: ::kernel::threads::WorkerThread,
	ping_replies: RingBuf<(Address,Address, Vec<u8>)>,
	neighbor_advertisments: RingBuf<(Address, Address)>,
}
impl Default for PendingPackets {
	fn default() -> Self {
		Self {
			ping_replies: RingBuf::new(128),
			neighbor_advertisments: RingBuf::new(128),
			_worker: ::kernel::threads::WorkerThread::new("ICMPv6", worker),
		}
	}
}
/// A worker thread used to handle sending ICMP replies without blocking the RX threads
fn worker() {
	loop {
		WORKER_SLEEP.sleep();

		// Some slight gymnastics to avoid holding the lock while sending packets
		enum Packet {
			NA {
				source: Address,
				destination: Address,
			},
			Ping {
				source: Address,
				destination: Address,
				data: Vec<u8>,
			}
		}
		fn get_packet(pp: &mut PendingPackets) -> Option<Packet> {
			if let Some((local, remote)) = pp.neighbor_advertisments.pop_front() {
				return Some(Packet::NA { source: local, destination: remote });
			}
			if let Some((local, remote, all_data)) = pp.ping_replies.pop_front() {
				return Some(Packet::Ping { source: local, destination: remote, data: all_data })
			}
			None
		}
		while let Some(p) = { let mut lh = PENDING_PACKETS.lock(); get_packet(&mut lh) }
		{
			match p {
			Packet::NA { source, destination } => {
				let mut pkt = [
					136, 0,
					0,0,
					4,0,0,0,	// Flags - set Override
					0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0,	// Target address
				];
				pkt[8..][..16].copy_from_slice(&source.to_bytes());
				let cksum = super::calculate_inner_checksum_it( NEXT_HEADER, source, destination, pkt.iter().copied());
				pkt[2..][..2].copy_from_slice( &u16::to_be_bytes(cksum) );
				let pkt = crate::nic::SparsePacket::new_root(&pkt);
				::kernel::futures::block_on(super::send_packet(source, destination, NEXT_HEADER, pkt));
			},
			Packet::Ping { source, destination, data } => {
				let mut hdr = [
					129, 0,
					0,0,
				];
				let cksum = super::calculate_inner_checksum_it( NEXT_HEADER, source, destination, hdr.iter().chain(data.iter()).copied());
				hdr[2..][..2].copy_from_slice( &u16::to_be_bytes(cksum) );

				let data = crate::nic::SparsePacket::new_root(&data);
				let pkt = crate::nic::SparsePacket::new_chained(&hdr, &data);
				::kernel::futures::block_on(super::send_packet(source, destination, NEXT_HEADER, pkt));
			},
			}
		}
	}
}

pub fn handle_packet(interface: &super::Interface, source: Address, destination: Address, mut reader: crate::nic::PacketReader<'_>) -> Result<(),()> {
	if super::calculate_inner_checksum_rdr( NEXT_HEADER, source, destination, reader.clone()) != 0 {
		// Uh-oh, checksum failure
	}
	let ty = reader.read_u8()?;
	let code = reader.read_u8()?;
	let _checksum = reader.read_u16n()?;

	match ty {
	1 => {	// Destination unreachable
		let _unused = reader.read_u32n();
		let code = match code {
			0 => CodeUnreachable::NoRoute,
			1 => CodeUnreachable::Disallowed,
			2 => CodeUnreachable::_NotAssigned2,
			3 => CodeUnreachable::AddressUnreachable,
			4 => CodeUnreachable::PortUnreachable,
			_ => {
				log_error!("Unknown ICMPv6 Unreachable code: {}", code);
				return Err( () );
			}
		};
		let _ = code;
	},
	2 => {	// Packet too big
	},
	3 => {	// "Time Exceeded"
		match code {
		0 => {	// Hop limit exceeded
		}
		1 => {	// Reassembly timed out
		}
		_ => {},
		}
	},
	4 => {	// Parameter Problem
	},
	128 => {	// Echo Request
		let mut v = vec![0; reader.remain()];
		reader.read(&mut v)?;
		match PENDING_PACKETS.lock().ping_replies.push_back((source, destination, v)) {
		Ok(_) => {},
		Err(_) => {},
		}
		WORKER_SLEEP.post();
	},
	129 => {	// Echo Reply
		let identifier = reader.read_u16n()?;
		let sequence_number = reader.read_u16n()?;
		// Check pending pings, poke the originator
		if let Some(v) = RUNNING_PINGS.read().get(&(source, identifier) ) {
			v.lock().push(sequence_number, ::kernel::time::ticks());
		}
	},

	133 => {	// Router Solicitation
	},
	134 => {	// Router Advertisement
	},
	135 => {	// Neighbour solicitation
		// Check if the target address matches us, and if it does - generate a reply
		let _resvd = reader.read_u32n()?;
		let target = Address::from_reader(&mut reader)?;
		let mut source_mac = Default::default();
		for opt in super::headers::OptionsIter::<NdOption>::new(&mut reader) {
			match opt {
			NdOption::TargetLinkLayerAddress(addr) => source_mac = addr,
			_ => {},
			}
		}
		if target == interface.addr() {
			// This is aimed at us

			// Learn the source address
			super::nd::learn(interface.local_mac, source_mac, target, super::nd::LearnSource::Snoop);

			// Then schedule a reply
			let _ = PENDING_PACKETS.lock().neighbor_advertisments.push_back((interface.addr(), target));
			WORKER_SLEEP.post();
		}
	},
	136 => {	// Neighbour advertisement
		// Nothing to do, we've already sniffed the result?
		let flags = reader.read_u32n()?;
		let _is_router = flags & (1 << 0) != 0;
		let _is_solicited = flags & (1 << 1) != 0;
		let is_override = flags & (1 << 2) != 0;
		let target = Address::from_reader(&mut reader)?;

		let mut source_mac = Default::default();
		for opt in super::headers::OptionsIter::<NdOption>::new(&mut reader) {
			match opt {
			NdOption::TargetLinkLayerAddress(addr) => source_mac = addr,
			_ => {},
			}
		}

		super::nd::learn(interface.local_mac, source_mac, target, if is_override { super::nd::LearnSource::Override } else { super::nd::LearnSource::Soft });
	},
	_ => {},
	}
	Ok( () )
}

enum CodeUnreachable {
	/// No entry in a routing table in a forwarding router
	NoRoute,
	/// A firewall has blocked the packet
	Disallowed,
	/// Explicitly not assigned
	_NotAssigned2,
	/// Unable to resolve the address to a link-layer address (or something similar)
	AddressUnreachable,
	/// No open port
	PortUnreachable,
}

struct PingState {
	replies: VecMap<u16, ::kernel::time::TickCount>,
	waiters: Vec<::kernel::threads::SleepObjectRef>,
}
impl PingState {
	fn push(&mut self, sequence: u16, time: ::kernel::time::TickCount) {
		self.replies.insert(sequence, time);
		for w in &self.waiters {
			w.signal();
		}
	}
}

#[allow(dead_code)]
enum NdOption {
	Unknown(super::headers::UnknownOptTy, u8),
	SourceLinkLayerAddress([u8; 6]),
	TargetLinkLayerAddress([u8; 6]),
	PrefixInformation {
		// These are not needed by the kernel
		//valid_lifetime: u32,
		//preferred_lifetime: u32,
		//prefix_length: u8,
		//prefix: Address,
		//on_link: bool,
		//auto_config: bool,
	},
	RedirectedHeader,
	MTU(u32),
}
impl super::headers::Opt for NdOption {
	fn from_value(code: u8, mut reader: super::headers::OptReader) -> Option<Self> {
		Some(match code {
		1 => Self::SourceLinkLayerAddress(reader.read_bytes_pad()?),
		2 => Self::TargetLinkLayerAddress(reader.read_bytes_pad()?),
		3 => Self::PrefixInformation { },
		4 => Self::RedirectedHeader { },
		5 => Self::MTU(reader.read_u32()?),
		_ => return None,
		})
	}

	fn unknown(t: super::headers::UnknownOptTy, code: u8, _data: [u8; 14]) -> Self {
		Self::Unknown(t, code)
	}
}