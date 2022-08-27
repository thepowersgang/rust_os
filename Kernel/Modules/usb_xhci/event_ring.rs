
#[derive(Debug)]
pub enum Event {
    /// Completion status/event for a Transfer TRB
    Transfer {
        /// Either some inline data, or a pointer to the TRB
        data: crate::hw::structs::TrbNormalData,
        /// Residual of the transfer length (i.e. number of bytes not transferred)
        transfer_length: u32,
        /// Completion code: 1 = Success
        completion_code: crate::hw::structs::TrbCompletionCode,
        slot_id: u8,
        endpoint_id: u8,
    },
    /// Command completed
    CommandCompletion {
        trb_pointer: u64,
        completion_code: crate::hw::structs::TrbCompletionCode,
        param: u32,
        slot_id: u8,
        vf_id: u8,
    },
    /// Change to the status of a port
    PortStatusChange {
        /// Port ID that changed
        port_id: u8,
        /// Completion code, should be "Success"
        completion_code: u8,
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
    fn get_completion_code(v: u8) -> crate::hw::structs::TrbCompletionCode {
        crate::hw::structs::TrbCompletionCode::from_u8(v)
                .unwrap_or_else(|v| {
                    log_warning!("Unknown completion code: {}", v);
                    crate::hw::structs::TrbCompletionCode::Invalid
                })
    }
    fn from_trb(trb: crate::hw::structs::Trb) -> Self {
        use crate::hw::structs::TrbType;
        match TrbType::from_trb_word3(trb.word3)
        {
        // See 6.4.2.1
        Ok(TrbType::TransferEvent) => Event::Transfer {
            data: crate::hw::structs::TrbNormalData::from_words((trb.word3 >> 2) & 1 != 0, trb.word0, trb.word1),
            transfer_length: trb.word2 & 0xFF_FFFF,
            completion_code: Self::get_completion_code( (trb.word2 >> 24) as u8 ),
            slot_id: (trb.word3 >> 24) as u8,
            endpoint_id: (trb.word3 >> 16) as u8 & 0xF,
            },
        // See 6.4.2.2
        Ok(TrbType::CommandCompletionEvent) => Event::CommandCompletion {
            trb_pointer: trb.word0 as u64 | (trb.word1 as u64) << 32,
            param: trb.word2 & 0xFF_FFFF,
            completion_code: Self::get_completion_code( (trb.word2 >> 24) as u8 ),
            slot_id: (trb.word3 >> 24) as u8,
            vf_id: (trb.word3 >> 16) as u8 & 0xF,
            },
        // See 6.4.2.3
        Ok(TrbType::PortStatusChangeEvent) => Event::PortStatusChange { 
            port_id: (trb.word0 >> 24) as u8,
            completion_code: (trb.word2 >> 24) as u8,
            },
        Ok(TrbType::BandwidthRequestEvent) => Event::BandwidthRequest { },
        Ok(TrbType::DoorbellEvent) => Event::Doorbell { },
        Ok(TrbType::HostControllerEvent) => Event::HostController { },
        Ok(TrbType::DeviceNotificationEvent) => Event::DeviceNotification { },
        Ok(TrbType::MfindexWrapEvent) => Event::MfindexWrap { },
        _ => Event::Unk(trb),
        }
    }
}

pub struct EventRing<Index>
{
    index: Index,
    state: ::kernel::sync::Mutex<State>,
    waiter: ::kernel::futures::Condvar,
}
struct State {
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
            regs.set_iman(3);   // Clear pending, and set IE
            regs.set_erstsz(1);
            regs.set_erstba(::kernel::memory::virt::get_phys(erst) as u64);
            regs.set_erdp(::kernel::memory::virt::get_phys(&ring_page[1]) as u64);
        }
        Ok(EventRing {
            index,
            waiter: Default::default(),
            state: ::kernel::sync::Mutex::new(State {
                ring_page,
                cycle_bit: false,
                read_ofs: 1,
                }),
        })
    }
    pub fn poll(&self, regs: &crate::hw::Regs) -> Option<Event> {
        let rv = self.state.lock().check(regs, self.index.into());
        if let Some(ref v) = rv {
            log_trace!("EventRing<{}>::poll: {:?}", self.index.into(), v);
        }
        rv
    }
    #[cfg(false_)]
    pub fn wait_sync(&self, regs: &crate::hw::Regs) -> Event {
        if let Some(v) = self.poll(regs) {
            return v;
        }
        loop {
            let k = self.waiter.get_key();
            if let Some(v) = self.poll(regs) {
                return v;
            }
            ::kernel::futures::block_on(self.waiter.wait(k));
        }
    }
    pub fn check_int(&self, regs: &crate::hw::Regs) {
        let regs = regs.interrupter(self.index.into());
        let v = regs.erdp();
        if v & 1<<3 != 0 {
            log_trace!("EventRing<{}>::check_int: updated", self.index.into());
            self.waiter.wake_all()
        }
    }
}

impl State {
    fn check(&mut self, regs: &crate::hw::Regs, index: u16) -> Option<Event> {
        let regs = regs.interrupter(index);
        let w3 = self.ring_page[self.read_ofs as usize].word3;
        if ((w3 & 1) == 0) != self.cycle_bit {
            log_trace!("check: Nothing");
            return None;
        }
        let d = self.ring_page[self.read_ofs as usize];
        self.read_ofs += 1;
        if self.read_ofs as usize == self.ring_page.len() {
            self.read_ofs = 1;
            self.cycle_bit = !self.cycle_bit;
        }
        unsafe {
            // Write `ERDP` with the last read address, ACKing the interrupt if needed
            regs.set_erdp(::kernel::memory::virt::get_phys(&self.ring_page[self.read_ofs as usize]) as u64 | (1<<3));
            // ACK the interrupt in IMAN
            regs.set_iman(regs.iman());
        }
        let rv = Event::from_trb(d);
        log_trace!("check: {:?}", rv);
        Some( rv )
    }
}