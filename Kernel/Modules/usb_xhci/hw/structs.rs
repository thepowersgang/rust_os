
// NOTE: device contexts are 0x40 + n*0x20

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum TrbType {
	_Reserved,
	Normal,
	SetupStage,
	DataStage,
	StatusStage,
	Isoch,
	Link,   // Command/TR
	EventData,
	NoOp,
	EnableSlotCommand,
	DisableSlotCommand,
	AddressDeviceCommand,
	ConfigureEndpointCommand,
	EvaluateContextCommand,
	ResetEndpointCommand,
	StopEndpointCommand,
	SetTrDequeuePointerCommand,
	ResetDeviceCommand,
	ForceEventCommand,
	NegotiateBandwidthCommand,
	SetLatencyToleranceValueCommand,
	GetPortBandwidthCommand,
	ForceHeaderCommand,
	NoOpCommand,
	GetExtendedPropertyCommand,
	SetExtendedPropertyCommand,
	// 26 -- 31 reserved
	TransferEvent = 32,
	CommandCompletionEvent,
	PortStatusChangeEvent,
	BandwidthRequestEvent,
	DoorbellEvent,
	HostControllerEvent,
	DeviceNotificationEvent,
	MfindexWrapEvent,
	// 40 -- 47 reserved
	// 48 -- 63 vendor defined
}
impl TrbType {
	pub fn from_trb_word3(v: u32) -> Result<Self,u8> {
		let v = ((v >> 10) & 63) as u8;
		Ok(match v {
		0 ..= 25
		// SAFE: Same repr
		| 32 ..= 39 => unsafe { ::core::mem::transmute(v) },
		_ => return Err(v),
		})
	}
	pub fn to_word3(self, cycle_bit: bool) -> u32 {
		let cycle = if cycle_bit { 1 } else { 0 };
		((self as u8 as u32) << 10) | cycle
	}
}

#[repr(u8)]
#[derive(Debug)]
pub enum TrbCompletionCode {
	Invalid,
	Success,
	DataBufferError,
	BabbleDetectedError,
	UsbTransactionError,
	TrbError,
	StallError,
	ResourceError,
	BandwidthError,
}
impl TrbCompletionCode {
	pub fn from_u8(v: u8) -> Result<TrbCompletionCode,u8> {
		match v {
		0 => Ok(Self::Invalid),
		1 => Ok(Self::Success),
		2 => Ok(Self::DataBufferError),
		3 => Ok(Self::BabbleDetectedError),
		4 => Ok(Self::UsbTransactionError),
		5 => Ok(Self::TrbError),
		6 => Ok(Self::StallError),
		7 => Ok(Self::ResourceError),
		8 => Ok(Self::BandwidthError),
		_ => Err(v),
		}
	}
}

/// Generic TRB (Transfer Buffer)
#[derive(Copy,Clone,Debug)]
#[repr(C)]
pub struct Trb
{
	pub word0: u32,
	pub word1: u32,
	pub word2: u32,
	/// Bits 15:0 are the type and state
	/// - Bit 0 is the cycle bit
	/// - Bits 10:15 are the type
	// Contains the type, must be written last
	pub word3: u32,
}
impl Trb {
	pub (crate) fn set_cycle(&mut self, cycle: bool) {
		self.word3 = (self.word3 & !1) | (cycle as u32);
	}
}
pub(crate) trait IntoTrb {
	fn into_trb(self, cycle: bool) -> Trb;
}

/// A linking TRB - used to chain buffers together and loop a ring buffer around
pub struct TrbLink
{
	/// Next base address
	pub addr: u64,
	/// Target for an IOC
	pub interrupter_target: u16,
	/// Causes the HC to switch its cycle bit
	pub toggle_cycle: bool,
	/// TODO? Does this do anything?
	pub chain: bool,
	/// Generate an interrupt when completed
	pub ioc: bool,
}
impl TrbLink {
	/// Construct a new link TRB that loops the ring buffer back around
	pub unsafe fn new_loopback(addr: u64) -> Self {
		TrbLink { addr, toggle_cycle: true, interrupter_target: 0, chain: false, ioc: false }
	}
}
impl IntoTrb for TrbLink {
	fn into_trb(self, cycle: bool) -> Trb {
		Trb {
			word0: (self.addr >>  0) as u32,
			word1: (self.addr >> 32) as u32,
			word2: 0
				| (self.interrupter_target as u32) << 22
				,
			word3: TrbType::Link.to_word3(cycle)
				| (self.toggle_cycle as u32) << 1
				| (self.chain as u32) << 4
				| (self.ioc as u32) << 5
				,
		}
	}
}

/// Indicates that a TRB is for use on the transfer queues
pub(crate) trait TransferTrb: IntoTrb {}

/// Data field of a normal TRB
pub enum TrbNormalData {
	/// A hardware pointer
	Pointer(u64),
	// Only valid for OUT endpoints (and only when MPS>=8?)
	InlineData([u8; 8]),
}
impl ::core::fmt::Debug for TrbNormalData {
	fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
		match *self {
		TrbNormalData::Pointer(v) => write!(f, "{:#x}", v),
		TrbNormalData::InlineData(v) =>
			write!(f, "{:02x} {:02x} {:02x} {:02x}  {:02x} {:02x} {:02x} {:02x}",
				v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],
				),
		}
	}
}
impl TrbNormalData {
	pub fn from_words(ed: bool, word0: u32, word1: u32) -> Self {
		if ed {
			TrbNormalData::InlineData( ((word0 as u64) | (word1 as u64) << 32).to_le_bytes() )
		}
		else {
			TrbNormalData::Pointer( (word0 as u64) | (word1 as u64) << 32 )
		}
	}
	pub fn make_inline(input: &[u8]) -> Option<TrbNormalData> {
		if input.len() <= 8 {
			let mut data = [0; 8];
			data[..input.len()].copy_from_slice(input);
			Some( TrbNormalData::InlineData(data) )
		}
		else {
			None
		}
	}
	fn to_word0(&self) -> u32 {
		match *self {
		TrbNormalData::Pointer(v) => v as u32,
		TrbNormalData::InlineData(v) => u32::from_le_bytes(::core::convert::TryInto::try_into(&v[..4]).unwrap()),
		}
	}
	fn to_word1(&self) -> u32 {
		match *self {
		TrbNormalData::Pointer(v) => (v >> 32) as u32,
		TrbNormalData::InlineData(v) => u32::from_le_bytes(::core::convert::TryInto::try_into(&v[4..]).unwrap()),
		}
	}
	fn is_immediate(&self) -> bool {
		match *self {
		TrbNormalData::Pointer(_) => false,
		TrbNormalData::InlineData(_) => true,
		}
	}
}

#[derive(Debug)]
/// A "Normal" TRB - Used for bulk and interrupt endpoints
pub struct TrbNormal
{
	pub data: TrbNormalData,
	// Word2
	pub transfer_length: u32,   // 17 bits
	pub td_size: u8,    // 5 bits
	pub interrupter_target: u16,
	// Word3
	/// xHC should evaluate the next TRB in the queue before saving state
	pub evaluate_next_trb: bool,
	/// Force an interrupt if a short packet is encountered
	pub interrupt_on_short_packet: bool,
	/// (TODO) Something about cache snooping
	pub no_snoop: bool,
	/// Associate this TRB with the next TRB in the ring (?if using this for scatter-gather?)
	pub chain_bit: bool,
	/// Interrupt On Completion - Generate an event when this TRB is retired
	pub ioc: bool,
	///// Indicates that the `data_buffer` field is raw data, and not a pointer
	//pub immediate_data: bool,
	pub block_event_interrupt: bool,    
}
impl TransferTrb for TrbNormal {
}
impl IntoTrb for TrbNormal {
	fn into_trb(self, cycle: bool) -> Trb {
		Trb {
			word0: self.data.to_word0(),
			word1: self.data.to_word1(),
			word2: 0
				| (self.transfer_length as u32 & 0x1FFFF) << 0
				| (self.td_size as u32 & 0x1F) << 17
				| (self.interrupter_target as u32) << 22
				,
			word3: TrbType::Normal.to_word3(cycle)
				| (self.evaluate_next_trb as u32) << 1
				| (self.interrupt_on_short_packet as u32) << 2
				| (self.no_snoop as u32) << 3
				| (self.chain_bit as u32) << 4
				| (self.ioc as u32) << 5
				| (self.data.is_immediate() as u32) << 6
				| (self.block_event_interrupt as u32) << 9
				,
		}
	}
}

/// TRB for a SETUP packet
pub struct TrbControlSetup
{
	// Word0
	pub bm_request_type: u8,
	pub b_request: u8,
	pub w_value: u16,

	// Word1
	pub w_index: u16,
	pub w_length: u16,

	// Word2
	pub trb_transfer_length: u32,
	pub interupter_target: u8,
	// Word3
	pub ioc: bool,
	pub idt: bool,
	pub transfer_type: TrbControlSetupTransferType,
}
#[repr(u8)]
#[derive(Debug)]
pub enum TrbControlSetupTransferType {
	#[allow(dead_code)]
	NoData = 0,
	_Reserved = 1,
	Out = 2,
	In = 3,
}
impl TransferTrb for TrbControlSetup {
}
impl IntoTrb for TrbControlSetup {
	fn into_trb(self, cycle: bool) -> Trb {
		Trb {
			word0: 0
				| (self.bm_request_type as u32) << 0
				| (self.b_request as u32) << 8
				| (self.w_value as u32) << 16
				,
			word1: 0
				| (self.w_index as u32) << 0
				| (self.w_length as u32) << 16
				, 
			word2: 0
				| (self.trb_transfer_length as u32 & 0x1FFFF) << 0
				| (self.interupter_target as u32) << 22
				,
			word3: TrbType::SetupStage.to_word3(cycle)
				| (self.ioc as u32) << 5
				| (self.idt as u32) << 6
				| (self.transfer_type as u32) << 16
				,
		}
	}
}

/// TRB for the data stage of a control transfer
pub struct TrbControlData
{
	pub data: TrbNormalData,
	/// Length of the data
	pub trb_transfer_length: u32,   // 17 bits
	pub td_size: u8,    // 5 bits
	pub interrupter_target: u16,
	
	/// xHC should evaluate the next TRB in the queue before saving state
	pub evaluate_next_trb: bool,
	/// Force an interrupt if a short packet is encountered
	pub interrupt_on_short_packet: bool,
	/// (TODO) Something about cache snooping
	pub no_snoop: bool,
	/// Associate this TRB with the next TRB in the ring (?if using this for scatter-gather?)
	pub chain_bit: bool,
	/// Interrupt On Completion - Generate an event when this TRB is retired
	pub ioc: bool,
	///// Indicates that the `data_buffer` field is raw data, and not a pointer
	//pub immediate_data: bool,
	/// Direction of the transfer
	pub direction_in: bool,
}
impl TransferTrb for TrbControlData {
}
impl IntoTrb for TrbControlData {
	fn into_trb(self, cycle: bool) -> Trb {
		Trb {
			word0: self.data.to_word0(),
			word1: self.data.to_word1(),
			word2: 0
				| (self.trb_transfer_length as u32 & 0x1FFFF) << 0
				| (self.td_size as u32 & 0x1F) << 17
				| (self.interrupter_target as u32) << 22
				,
			word3: TrbType::DataStage.to_word3(cycle)
				| (self.evaluate_next_trb as u32) << 1
				| (self.interrupt_on_short_packet as u32) << 2
				| (self.no_snoop as u32) << 3
				| (self.chain_bit as u32) << 4
				| (self.ioc as u32) << 5
				| (self.data.is_immediate() as u32) << 6
				| (self.direction_in as u32) << 16
				,
		}
	}
}

pub struct TrbControlStatus
{
	pub interrupter_target: u16,
	
	/// xHC should evaluate the next TRB in the queue before saving state
	pub evaluate_next_trb: bool,
	/// Interrupt On Completion - Generate an event when this TRB is retired
	pub ioc: bool,
	/// Direction of the transfer
	pub direction_in: bool,
}
impl TransferTrb for TrbControlStatus {
}
impl IntoTrb for TrbControlStatus {
	fn into_trb(self, cycle: bool) -> Trb {
		Trb {
			word0: 0,
			word1: 0,
			word2: 0
				| (self.interrupter_target as u32) << 22
				,
			word3: TrbType::StatusStage.to_word3(cycle)
				| (self.evaluate_next_trb as u32) << 1
				| (self.ioc as u32) << 5
				| (self.direction_in as u32) << 16
				,
		}
	}
}

// --------------------------------------------------------------------
// Device context definitions
// --------------------------------------------------------------------

/// Complete structure for an input context (with control, slot, and endpoints)
#[repr(C)]
pub struct AddrInputContext {
	pub ctrl: InputControlContext,
	pub slot: SlotContext,
	pub eps: [EndpointContext; 31],
}

#[derive(Copy,Clone,Debug)]
#[repr(C)]
// 6.2.5.1
/// Input control context - specifies the details of the endpoints being configured
pub struct InputControlContext
{
	/// Bitmap of device context entries to be disabled by this command
	pub drop_context_flags: u32,
	/// Bitmap of device context entries to be added/enabled
	pub add_context_flags: u32,
	_resvd: [u32; 5],
	/// (ConfigureEndpoint)
	pub configuration_value: u8,
	/// (ConfigureEndpoint)
	pub interface_number: u8,
	/// (ConfigureEndpoint)
	pub alternate_setting: u8,
	_resvd2: u8,
}   // sizeof = 8 words
impl InputControlContext
{
	pub fn zeroed() -> Self {
		InputControlContext {
			drop_context_flags: 0,
			add_context_flags: 0,
			_resvd: [0; 5],
			configuration_value: 0,
			interface_number: 0,
			alternate_setting: 0,
			_resvd2: 0,
		}
	}
}

#[derive(Copy,Clone,Debug)]
#[repr(C)]
/// Header for a device context
// 6.2.2
pub struct SlotContext
{
	/// 19:0 - Route string (See USB3 spec 8.9). A sequence of 4-bit port numbers
	/// 23:20 - Speed (same values as PORTSC)
	/// 25 - Multi-TT (MTT)
	/// 26 - Hub
	/// 31:27 - Context Entries
	pub word0: u32,
	// 15:0 - Max Exit Latency
	pub word1: u32,
	// 7:0 - USB Device Address
	// 31:17 - Slot State
	pub word2: u32,
	pub word3: u32,
	_resvd: [u32; 4],
}   // sizeof = 8 words
impl SlotContext {
	pub fn new(words: [u32; 4]) -> SlotContext {
		SlotContext { word0: words[0], word1: words[1], word2: words[2], word3: words[3], _resvd: [0; 4], }
	}
}

#[repr(u8)]
pub enum EndpointType {
	_Reserved,
	IsochOut,
	BulkOut,
	_InterruptOut,
	Control,
	IsochIn,
	BulkIn,
	InterruptIn,
}

// 6.2.3 "Endpoint Context"
#[derive(Copy,Clone,Debug)]
#[repr(C)]
pub struct EndpointContext
{
	/// 2:0 - Endpoint state
	/// 9:8 - Mult - (Isoch, iff `HCCPARAMS2.LEC`) - Number of bursts per interval
	/// 14:10 - MaxPStreams - Number of primary streams
	/// 15 - Linear Stream Array - Disables the use of secondary stream arrays
	/// 23:16 - Interval - Period between successive requests, as 125us ** <value>
	/// 31:24 - Max Endpoint Service Time Interval Payload High (see `HCCPARAMS2.LEC`)
	pub word0: u32,
	/// 2:1 - CErr
	/// 5:3 - Endpoint type
	/// - 0 = Not Valid
	/// - 1 = Isoch Out
	/// - 2 = Bulk Out
	/// - 3 = Interrupt Out
	/// - 4 = Control
	/// - 5 = Isoch In
	/// - 6 = Bulk In
	/// - 7 = Interrupt In
	/// 7 - Host Initiate Disable
	/// 15:8 - Max Burst Size
	/// 31:16 - Max Packet Size
	pub word1: u32,
	/// 0 - Dequeue Cycle State
	/// 63:4 - TR Dequeue Pointer (or stream context array)
	pub tr_dequeue_ptr: u64,
	/// 15:0 - Average TRB Length
	/// 31:16 - Max Endpoint Service Time Interval Payload Low
	pub word4: u32,
	_resvd: [u32; 3],
}   // sizeof = 8 words
impl EndpointContext
{
	pub fn zeroed() -> Self {
		EndpointContext {
			word0: 0,
			word1: 0,
			tr_dequeue_ptr: 0,
			word4: 0,
			_resvd: [0; 3],
		}
	}

	pub fn set_word1(&mut self, ty: EndpointType, max_packet_size: u16) {
		self.word1 = (max_packet_size as u32) << 16 | (ty as u8 as u32) << 3;
	}
}


