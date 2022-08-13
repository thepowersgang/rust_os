
#[derive(Debug)]
pub enum Command
{
    Nop,
    EnableSlot,
    DisableSlot {
        slot_idx: u8,
    },
    // 4.6.5 "Address Device"
    AddressDevice {
        /// Slot to be used for this device
        slot_idx: u8,
        /// Pointer to the "Input Context" structure
        input_context_pointer: u64,
        /// Do everything BUT send the SET_ADDRESS message on the bus
        block_set_address: bool,
    },
    ConfigureEndpoint {
        // ... TODO
    }
}
impl Command
{
    fn to_desc(self, cycle_bit: bool) -> crate::hw::structs::Trb {
        use crate::hw::structs::TrbType;
        match self
        {
        Command::Nop => crate::hw::structs::Trb
            {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: TrbType::NoOpCommand.to_word3(cycle_bit),
            },
        // 6.4.3.2
        Command::EnableSlot => crate::hw::structs::Trb
            {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: TrbType::EnableSlotCommand.to_word3(cycle_bit),
            },
        // 6.4.3.3
        Command::DisableSlot { slot_idx } => crate::hw::structs::Trb
            {
            word0: 0,
            word1: 0,
            word2: 0,
            word3: (slot_idx as u32) << 24 | TrbType::DisableSlotCommand.to_word3(cycle_bit),
            },
        // 6.4.3.4
        Command::AddressDevice { slot_idx, input_context_pointer, block_set_address } => crate::hw::structs::Trb
            {
            word0: (input_context_pointer & 0xFFFF_FFF0) as u32,
            word1: (input_context_pointer >> 32) as u32,
            word2: 0,
            word3: (slot_idx as u32) << 24
                | if block_set_address { 1 << 9 } else { 0 }
                | TrbType::AddressDeviceCommand.to_word3(cycle_bit),
            },
        _ => todo!("Command::to_desc: {:?}", self),
        }
    }
}

pub struct CommandRing
{
    ring_page: ::kernel::memory::virt::ArrayHandle<crate::hw::structs::Trb>,
    offset: usize,
    cycle_bit: bool,
}

// See 4.9.3 "Command Ring Management"
// To send a command
// - Push a command to the command ring
//  > On each push, ensure that the `cycle_bit` matches the CCS value read from the controller (CRCR.RCS)
// - Write to the command doorbell (offset 0 in the doorbell registers)
impl CommandRing
{
    pub fn new(regs: &crate::hw::Regs) -> Result<Self,::kernel::device_manager::DriverBindError> {
        let ring_page = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?.into_array();
        unsafe {
            regs.set_crcr(::kernel::memory::virt::get_phys(&ring_page[0]) as u64);
        }
        Ok(CommandRing {
            ring_page,
            offset: 0,
            cycle_bit: true,
        })
    }
    pub fn enqueue_command(&mut self, regs: &crate::hw::Regs, command: Command) {
        log_debug!("enqueue_command: {:?}", command);
        // 1. Read CRCR to ensure that the ring isn't full
        let crcr = regs.crcr();
        let ctrlr_cycle_bit = (crcr & 1) == 1;
        // TODO: Check for full
        // 2. Write a new entry to the ring 
        let command_desc = command.to_desc(self.cycle_bit);
        self.ring_page[self.offset] = command_desc;
        self.offset += 1;
        if self.offset == self.ring_page.len() {
            self.cycle_bit = !self.cycle_bit;
            self.offset = 0;
        }
        regs.ring_doorbell(0, 0);
    }

    pub fn get_command_type(&self, addr: u64) -> Option<crate::hw::structs::TrbType> {
        let ofs = addr.checked_sub(::kernel::memory::virt::get_phys(&self.ring_page[0]) as u64)?;
        let idx = ofs / ::core::mem::size_of::<crate::hw::structs::Trb>() as u64;
        if idx >= self.ring_page.len() as u64 {
            return None;
        }
        crate::hw::structs::TrbType::from_trb_word3(self.ring_page[idx as usize].word3).ok()
    }
}