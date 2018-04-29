//
//!

#[repr(usize)]
#[allow(dead_code)]
#[derive(Debug)]
pub enum Regs
{
	HcRevision = 0,
	HcControl,
	HcCommandStatus,
	HcInterruptStatus,
	HcInterruptEnable,
	HcInterruptDisable,

	HcHCCA,
	HcPeriodCurrentED,
	HcControlHeadED,
	HcControlCurrentED,
	HcBulkHeadED,
	HcBulkCurrentED,
	HcDoneHead,

	HcFmInterval,
	HcFmRemaining,
	HcFmNumber,
	HcPeriodicStart,
	HcLSThreshold,

	//  0: 7 = NDP (Max of 15)
	HcRhDescriptorA,
	HcRhDescriptorB,
	HcRhStatus,
	HcRhPortStatus0,
	HcRhPortStatus1,
	HcRhPortStatus2,
	HcRhPortStatus3,
	HcRhPortStatus4,
	HcRhPortStatus5,
	HcRhPortStatus6,
	HcRhPortStatus7,
	HcRhPortStatus8,
	HcRhPortStatus9,
	HcRhPortStatus10,
	HcRhPortStatus11,
	HcRhPortStatus12,
	HcRhPortStatus13,
	HcRhPortStatus14,
	HcRhPortStatus15,
}

// Host Controller Communication Area
// 256 bytes total
#[repr(C)]
pub struct Hcca
{
	pub interrupt_table: [u32; 128 / 4],
	pub frame_number: u16,
	_pad1: u16,
	pub done_head: u32,
	_reserved: [u32; 116 / 4],
}

// Size: 16 bytes, fits 256 per page - i.e just enough for a u8
#[repr(C)]
pub struct Endpoint
{
	//  0: 6 = Address
	//  7:10 = Endpoint Num
	// 11:12 = Direction (TD, OUT, IN, TD)
	// 13    = Speed (Full, Low)
	// 14    = Skip entry
	// 15    = Format (Others, Isochronous)
	// 16:26 = Max Packet Size
	// 27:30 = AVAIL
	// 31    = AVAIL
	pub flags: u32,
	//  0: 3 = AVAIL
	//  4:31 = TailP
	pub tail_ptr: u32,
	//  0    = Halted (Queue stopped due to error)
	//  1    = Data toggle carry
	//  2: 3 = ZERO
	//  4:31 = HeadP
	pub head_ptr: u32,
	//  0: 3 = AVAIL
	//  4:31 = NextED
	pub next_ed: u32,	// Next endpoint descriptor in the chain.

	// TODO: Extra metadata?
}

/// A general (non-isochronous) transfer descriptor
#[repr(C)]
pub struct GeneralTD
{
	/// Flags
	//  0:17 = AVAIL
	//       > 0: Allocated bit (1 when allocated)
	// 18    = Buffer Rounding (Allow an undersized packet)
	// 19:20 = Direction (SETUP, OUT, IN, Resvd)
	// 21:23 = Delay Interrupt (Frame count, 7 = no int)
	// 24:25 = Data Toggle (ToggleCarry, ToggleCarry, 0, 1)
	// 26:27 = Error Count
	// 28:31 = Condition Code
	pub flags: u32,

	// Base address of packet (or current when being read)
	pub cbp: u32,

	/// Next transfer descriptor in the chain
	pub next_td: u32,

	/// Address of final byte in buffer
	// - Note, this can be in a different page to the base address to a maximum of two
	pub buffer_end: u32,

	// -- Acess Information
	pub meta_async_handle: u64,
	_meta_unused: u64,
}

// 32 * 16  = 512 bytes long
/// Structure of part of the HCCA (but NOT specified by the hardware, just suggested)
pub struct IntLists
{
	/// 16ms polling periods
	pub int_16ms: [Endpoint; 16],
	pub int_8ms: [Endpoint; 8],
	pub int_4ms: [Endpoint; 4],
	pub int_2ms: [Endpoint; 2],
	pub int_1ms: [Endpoint; 1],
	/// The end of any list
	pub stop_endpoint: Endpoint,
}


