//! Hardware structure definitions
//! 

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Regs
{
	ID0, ID1, ID2,
	ID3, ID4, ID5,

	MAR7 = 0x8,
	MAR6,
	MAR5,
	MAR4,
	MAR3,
	MAR2,
	MAR1,
	MAR0,
	DTCCR = 0x10,
	/// Transmit Normal Priority Descriptors: Start address (64-bit). (256-byte alignment)
	TNPDS = 0x20,
	/// Transmit High Priority Descriptors: Start address (64-bit). (256-byte alignment)
	THPDS = 0x28,

	CR = 0x37,
	TPPoll = 0x38,
	IMR = 0x3C,
	ISR = 0x3E,
	TCR = 0x40,
	RCR = 0x44,
	/// 32-bit counter, 8ns (125MHz) ticks
	TCTR = 0x48,

	TimerInt = 0x58,

	/// Receive (Rx) Packet Maximum Size
	RMS = 0xDA,
	RDSAR = 0xE4,	// 8 bytes
	/// Max Transmit Packet Size
	MTPS = 0xEC,
}

pub const ISR_ROK: u16 = 0x01;
pub const ISR_TOK: u16 = 0x02;

pub const DESC0_LS: u32 = 1 << 28;
pub const DESC0_FS: u32 = 1 << 29;
pub const DESC0_EOR: u32 = 1 << 30;
pub const DESC0_OWN: u32 = 1 << 31;

/// Card-Owned Rx descriptor
pub struct RxDescOwn
{
	pub rx_buffer_addr: u64,
	pub buffer_length: u16,
}
impl RxDescOwn
{
	pub fn new(buffer: u64, len: u16) -> Self {
		RxDescOwn {
			rx_buffer_addr: buffer,
			buffer_length: len,
		}
	}
	pub fn into_array(self) -> [u32; 4] {
		[
			(self.buffer_length as u32) | DESC0_OWN,
			0,
			self.rx_buffer_addr as u32,
			(self.rx_buffer_addr >> 32) as u32,
		]
	}
}

pub struct RxDesc
{
	pub rx_buffer_addr: u64,
	pub buffer_length: u16,
}
impl RxDesc
{
	pub fn from_array(a: [u32; 4]) -> Self {
		Self {
			rx_buffer_addr: (a[3] as u64) << 32 | (a[2] as u64),
			buffer_length: a[0] as u16,
		}
	}
}

// Un-owned TX descriptor
pub struct TxDesc
{
	pub tx_buffer_addr: u64,
	pub frame_length: u16,
	pub flags3: u8,
	pub vlan_info: u16,
}
impl TxDesc
{
	pub fn into_array(&self) -> [u32; 4] {
		[
			(self.flags3 as u32) << 26 | (self.frame_length as u32),
			self.vlan_info as u32,
			self.tx_buffer_addr as u32,
			(self.tx_buffer_addr >> 32) as u32,
		]
	}
}