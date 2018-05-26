// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/tcp.rs
//! Transmission Control Protocol (Layer 4)

pub fn init()
{
	::ipv4::register_handler(6, rx_handler_v4);
}

fn rx_handler_v4(int: &::ipv4::Interface, src_addr: ::ipv4::Address, mut pkt: ::nic::PacketReader)
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

	// Search for active connections with this quad (or triple)
	//if let Some(c) = CONNECTIONS.get( &(src_addr, hdr.source_port, hdr.dest_port) )
	//{
	//}
	// If none found, look for servers on the destination (if SYN)
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

