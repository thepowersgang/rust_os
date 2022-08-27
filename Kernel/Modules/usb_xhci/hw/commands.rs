//! Commands

use crate::hw::structs::{Trb, IntoTrb, TrbType};

pub(crate) trait CommandTrb: ::core::fmt::Debug + IntoTrb {}

// 4.6.2 "No Op"
/// No-operation command - does nothing but emit a "command complete" event
#[derive(Debug)]
pub struct Nop;
impl CommandTrb for Nop {}
impl IntoTrb for Nop {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: TrbType::NoOpCommand.to_word3(cycle),
        }
    }
}

// 4.6.4 "Enable Slot"
/// Allocate a new device slot
#[derive(Debug)]
pub struct EnableSlot;
impl CommandTrb for EnableSlot {}
impl IntoTrb for EnableSlot {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: TrbType::EnableSlotCommand.to_word3(cycle),
        }
    }
}

// 4.6.4 "Disable Slot"
/// Release a device slot
#[derive(Debug)]
pub struct DisableSlot(u8);
impl DisableSlot {
    pub fn _new(slot_idx: u8) -> Self {
        DisableSlot(slot_idx)
    }
}
impl CommandTrb for DisableSlot {}
impl IntoTrb for DisableSlot {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: (self.0 as u32) << 24 | TrbType::DisableSlotCommand.to_word3(cycle),
        }
    }
}

// 4.6.5 "Address Device"
/// Allocate and set the address of the current #0 device
#[derive(Debug)]
pub struct AddressDevice {
    /// Slot to be used for this device
    slot_idx: u8,
    /// Pointer to the "Input Context" structure
    input_context_pointer: u64,
    /// Do everything BUT send the SET_ADDRESS message on the bus
    block_set_address: bool,
}
impl AddressDevice {
    pub unsafe fn new(slot_idx: u8, input_context_pointer: u64, block_set_address: bool) -> Self {
        Self { slot_idx, input_context_pointer, block_set_address }
    }
}
impl CommandTrb for AddressDevice {}
impl IntoTrb for AddressDevice {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: (self.input_context_pointer & 0xFFFF_FFF0) as u32,
            word1: (self.input_context_pointer >> 32) as u32,
            word2: 0,
            word3: (self.slot_idx as u32) << 24
                | if self.block_set_address { 1 << 9 } else { 0 }
                | TrbType::AddressDeviceCommand.to_word3(cycle),
        }
    }
}

// 4.6.6 "Configure Endpoint"
/// Allocate and set the address of the current #0 device
#[derive(Debug)]
pub struct ConfigureEndpoint {
    /// Slot to be used for this device
    slot_idx: u8,
    /// Pointer to the "Input Context" structure
    input_context_pointer: u64,
    /// Deconfigure the device/endpoints (dropping all endpoints)
    deconfigure: bool,
}
impl ConfigureEndpoint {
    pub unsafe fn new_configure(slot_idx: u8, input_context_pointer: u64) -> Self {
        Self { slot_idx, input_context_pointer, deconfigure: false }
    }
}
impl CommandTrb for ConfigureEndpoint {}
impl IntoTrb for ConfigureEndpoint {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: (self.input_context_pointer & 0xFFFF_FFF0) as u32,
            word1: (self.input_context_pointer >> 32) as u32,
            word2: 0,
            word3: (self.slot_idx as u32) << 24
                | if self.deconfigure { 1 << 9 } else { 0 }
                | TrbType::ConfigureEndpointCommand.to_word3(cycle),
        }
    }
}

// 4.6.6 "Evaluate Context"
/// Notifies the controller of changes to the device context fields
#[derive(Debug)]
pub struct EvaluateContext {
    /// Slot to be used for this device
    slot_idx: u8,
    /// Pointer to the "Input Context" structure
    input_context_pointer: u64,
}
impl EvaluateContext {
    pub unsafe fn _new(slot_idx: u8, input_context_pointer: u64) -> Self {
        Self { slot_idx, input_context_pointer }
    }
}
impl CommandTrb for EvaluateContext {}
impl IntoTrb for EvaluateContext {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: (self.input_context_pointer & 0xFFFF_FFF0) as u32,
            word1: (self.input_context_pointer >> 32) as u32,
            word2: 0,
            word3: (self.slot_idx as u32) << 24
                | TrbType::EvaluateContextCommand.to_word3(cycle),
        }
    }
}


// 4.6.8 "Reset Endpoint"
/// Used to recover from a halted condition on an endpoint
#[derive(Debug)]
pub struct ResetEndpoint
{
    /// Device slot index
    slot_idx: u8,
    /// Endpoint index
    endpoint_id: u8,
    /// Preserve transfer state, re-attempting the last transfer
    transfer_state_preserve: bool,
}
impl CommandTrb for ResetEndpoint {}
impl IntoTrb for ResetEndpoint {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: (self.slot_idx as u32) << 24
                | (self.endpoint_id as u32) << 16
                | (self.transfer_state_preserve as u32) << 9
                | TrbType::ResetEndpointCommand.to_word3(cycle),
        }
    }
}

// 4.6.9 "Stop Endpoint"
/// Stop all transfers on an endpoint (so the software can manipulate inside the transfer ring)
#[derive(Debug)]
pub struct StopEndpoint
{
    slot_idx: u8,
    endpoint_id: u8,
}
impl CommandTrb for StopEndpoint {}
impl IntoTrb for StopEndpoint {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: (self.slot_idx as u32) << 24
                | (self.endpoint_id as u32) << 16
                | TrbType::StopEndpointCommand.to_word3(cycle),
        }
    }
}


#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum StreamContextType {
    SecondaryTransferRing,
    PrimaryTransferRing,
    PrimarySsa8,
    PrimarySsa16,
    PrimarySsa32,
    PrimarySsa64,
    PrimarySsa128,
    PrimarySsa256,
}
#[derive(Debug)]
pub struct SetTrDequeuePointer
{
    slot_idx: u8,
    endpoint_id: u8,
    stream_id: u16,
    new_dequeue_pointer: u64,
    cycle: bool,
    stream_context_type: StreamContextType,
}
impl SetTrDequeuePointer {
    pub unsafe fn _new_bare(slot_idx: u8, endpoint_id: u8, new_dequeue_pointer: u64, cycle: bool) -> Self {
        Self { slot_idx, endpoint_id, new_dequeue_pointer, cycle, stream_id: 0, stream_context_type: StreamContextType::SecondaryTransferRing }
    }
    pub unsafe fn _new_streamed(slot_idx: u8, endpoint_id: u8, stream_id: u16, sct: StreamContextType, new_dequeue_pointer: u64, cycle: bool) -> Self {
        Self { slot_idx, endpoint_id, new_dequeue_pointer, cycle, stream_id, stream_context_type: sct }
    }
}
impl CommandTrb for SetTrDequeuePointer {}
impl IntoTrb for SetTrDequeuePointer {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: 0
                | (self.new_dequeue_pointer & 0xFFFF_FFF0) as u32
                | (self.stream_context_type as u32) << 1
                | self.cycle as u32
                ,
            word1: (self.new_dequeue_pointer >> 32) as u32,
            word2: (self.stream_id as u32) << 16,
            word3: (self.slot_idx as u32) << 24
                | (self.endpoint_id as u32) << 16
                | TrbType::SetTrDequeuePointerCommand.to_word3(cycle),
        }
    }
}

#[derive(Debug)]
pub struct ResetDevice(u8);
impl ResetDevice {
    pub fn _new(slot_idx: u8) -> Self {
        ResetDevice(slot_idx)
    }
}
impl CommandTrb for ResetDevice {}
impl IntoTrb for ResetDevice {
    fn into_trb(self, cycle: bool) -> Trb {
        Trb {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: (self.0 as u32) << 24 | TrbType::ResetDeviceCommand.to_word3(cycle),
        }
    }
}