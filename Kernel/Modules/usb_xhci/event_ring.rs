pub enum Event {
    /// Completion status/event for a Transfer TRB
    Transfer {
        trb_pointer: u64,
        transfer_length: u32,
        completion_code: u8,
        slot_id: u8,
        endpoint_id: u8,
        ed: bool,
    },
    CommandCompletion {

    },
    PortStatusChange {

    },
    BandwidthRequest {

    },
    Doorbell {
    },
    HostController {
    },
    DeviceNotification {
    },
    MfindexWrap {
    },
    Unk(crate::hw::structs::Trb),
}
impl Event {
    fn from_trb(trb: crate::hw::structs::Trb) -> Self {
        // See 6.4.2.1
        match (trb.word3 >> 10) & 0x3F
        {
        32 => Event::Transfer {
            trb_pointer: trb.word0 as u64 | (trb.word1 as u64) << 32,
            transfer_length: trb.word2 & 0xFF_FFFF,
            completion_code: (trb.word2 >> 24) as u8,
            slot_id: (trb.word3 >> 24) as u8,
            endpoint_id: (trb.word3 >> 16) as u8 & 0xF,
            ed: (trb.word3 >> 2) & 1 != 0,
            },
        33 => Event::CommandCompletion { },
        34 => Event::PortStatusChange { },
        35 => Event::BandwidthRequest { },
        36 => Event::Doorbell { },
        37 => Event::HostController { },
        38 => Event::DeviceNotification { },
        39 => Event::MfindexWrap { },
        _ => Event::Unk(trb),
        }
    }
}

pub struct EventRing<Index>
{
    index: Index,
    ring_page: ::kernel::memory::virt::ArrayHandle<crate::hw::structs::Trb>,
    cycle_bit: bool,
    read_ofs: u16,
}

#[derive(Copy,Clone)]
pub struct Zero;
impl Into<u16> for Zero { fn into(self) -> u16 { 0 } }

impl EventRing<Zero>
{
    pub fn new_zero(regs: &crate::hw::Regs) -> Result<Self,::kernel::device_manager::DriverBindError> {
        Self::new(regs, Zero)
    }
}
impl<Index> EventRing<Index>
where
    Index: Into<u16> + Copy
{
    pub fn new(regs: &crate::hw::Regs, index: Index) -> Result<Self,::kernel::device_manager::DriverBindError> {
        let regs = regs.interrupter(index.into());
        let mut ring_page = ::kernel::memory::virt::alloc_dma(64, 1, "usb_xhci")?.into_array();
        unsafe {
            // ERST entries are two 64-bit words (with a bunch of required zero fields)
            let erst: &mut [u64; 2] = &mut *(&mut ring_page[0] as *mut _ as *mut _);
            erst[0] = ::kernel::memory::virt::get_phys(&ring_page[1]) as u64;
            erst[1] = (ring_page.len() - 1) as u64;
            regs.set_iman(1);
            regs.set_erstba(::kernel::memory::virt::get_phys(&ring_page[0]) as u64);
            regs.set_erstsz(1);
            regs.set_erdp(::kernel::memory::virt::get_phys(&ring_page[1]) as u64);
        }
        Ok(EventRing {
            ring_page,
            index,
            cycle_bit: false,
            read_ofs: 1,
        })
    }
    pub fn check(&mut self, regs: &crate::hw::Regs) -> Option<Event> {
        let w3 = self.ring_page[self.read_ofs as usize].word3;
        if ((w3 & 1) == 0) != self.cycle_bit {
            return None;
        }
        let d = self.ring_page[self.read_ofs as usize];
        self.read_ofs += 1;
        if self.read_ofs as usize == self.ring_page.len() {
            self.read_ofs = 1;
            self.cycle_bit = !self.cycle_bit;
        }
        Some( Event::from_trb(d) )
    }
}