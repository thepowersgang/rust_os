
// NOTE: device contexts are 0x40 + n*0x20

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum TrbType {
    _Reserved,
    Normal,
    SetupState,
    DataStage,
    StatuStage,
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
    ForceHeaderCommmand,
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
        | 32 ..= 39 => unsafe { ::core::mem::transmute(v) },
        _ => return Err(v),
        })
    }
    pub fn to_word3(self, cycle_bit: bool) -> u32 {
        let cycle = if cycle_bit { 1 } else { 0 };
        ((self as u8 as u32) << 10) | cycle
    }
}

/// An 
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

#[cfg(false_)]
struct InputContext
{
    pub icc: InputControlContext,
    pub slot_context: SlotContext,
    pub ep0_context: EndpointContext,
    pub ep_contexts: [ [EndpointContext; 2]; 15 ],
}

#[derive(Copy,Clone,Debug)]
#[repr(C)]
// 6.2.5.1
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

#[derive(Copy,Clone,Debug)]
#[repr(C)]
pub struct EndpointContext
{
    /// 2:0 - Endpoint state
    /// 9:8 - Mult
    pub word0: u32,
    /// 5:3 - Endpoint type
    /// - 0 = Not Valid
    /// - 1 = Isoch Out
    /// - 2 = Bulk Out
    /// - 3 = Interrupt Out
    /// - 4 = Control
    /// - 5 = Isoch In
    /// - 6 = Bulk In
    /// - 7 = Interrupt In
    /// ...
    /// 31:16 - Max Packet Size
    pub word1: u32,
    /// 0 - Dequeue Cycle State
    /// 63:4 - TR Dequeue Pointer
    pub tr_dequeue_ptr: u64,
    /// 15:0 - Average TRB Length
    /// 31:16 - Max Endpoint Service Time Interrupt Payload Low
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
}