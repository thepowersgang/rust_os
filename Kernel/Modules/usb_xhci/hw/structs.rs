
// NOTE: device contexts are 0x40 + n*0x20

/// An 
#[derive(Copy,Clone)]
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

/// Header for a device context
pub struct SlotContext
{
    /// 19:0 - Route string
    pub word0: u32,
    pub word1: u32,
    pub word2: u32,
    pub word3: u32,
    _resvd: [u32; 4],
}

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
    pub word3: u32,
    _resvd: [u32; 3],
}
