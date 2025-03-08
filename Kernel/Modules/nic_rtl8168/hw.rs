//! Hardware structure definitions
//! 
use ::core::sync::atomic::Ordering;

#[repr(u8)]
#[derive(Copy, Clone)]
#[allow(dead_code)]
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
	/// Write a 64-bit value here to get the hardware to dump its tally registers to a 64-byte (size and align) memory range
	/// 
	/// Set bit 3 to start the dump, it's cleared once complete
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

	/// Triggers an interrupt when [Regs::TCTR] reaches this value
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

pub type DescArray = [::core::sync::atomic::AtomicU32; 4];

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
}
impl RxDesc
{
	pub fn get_len(a: &DescArray) -> usize {
		(a[0].load(Ordering::Relaxed) & 0xFFFF) as usize
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