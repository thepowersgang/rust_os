
pub enum Command
{
    Nop,
    EnableSlot,
    DisableSlot {
        slot_idx: u8,
    },
    AddressDevice {
        slot_idx: u8,
        address: u8,
        // ... TODO
    },
    ConfigureEndpoint {
        // ... TODO
    }
}
impl Command
{
    fn to_desc(self, cycle_bit: bool) -> crate::hw::structs::Trb {
        todo!("")
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
        // 1. Read CRCR to ensure that the ring isn't full
        let crcr = regs.crcr();
        let ctrlr_cycle_bit = (crcr & 1) == 1;
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
}