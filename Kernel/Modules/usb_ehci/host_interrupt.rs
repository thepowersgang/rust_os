//!
//! 
use crate::hw_structs;

impl super::HostInner
{
    pub(crate) fn add_qh_to_interrupt(&self, mut qh: crate::desc_pools::QhHandle, period: usize, mut td: crate::desc_pools::TdHandle) -> IntHandle
    {
        self.td_pool.get_data_mut(&mut td).token |= hw_structs::QTD_TOKEN_STS_ACTIVE | hw_structs::QTD_TOKEN_IOC;

        let mut pq = self.periodic_queue.lock();

        let period = period.clamp(1, 1024 / 8) * 8; // The periodic array is indexed by microframes, and has 1024 entries
        // Figure out a good offset (lowest load)
        let (best_load, best_ofs) = (0 .. period).map(|o| (
            // SAFE: Lock is held
            (0 .. 1024).step_by(period).map(|i| unsafe { self.intr_calculate_load(pq[o + i]) }).sum::<usize>(),
            o,
            )
            ).min().unwrap();
        log_debug!("add_qh_to_interrupt({qh:?}): period={period} uf best_ofs={best_ofs} (best_load={best_load})");
        // Visit each slot and find the entry that would be our next (equal or lower period)
        let mut next = 1;
        for s in (0 .. 1024).step_by(period).map(|s| best_ofs + s) {
            // SAFE: Lock is held
            let n = unsafe { self.intr_find_insert_point(pq[s], period).1 };
            if n != 1 {
                if next != 1 {
                    // If the `next` is already set, then it must agree
                    assert!(next == n);
                }
                else {
                    next = n;
                }
            }
        }
        self.qh_pool.get_data_mut(&mut qh).hlink = next;

        self.qh_pool.assign_td(&mut qh, &self.td_pool, td);

        let addr = self.qh_pool.get_phys(&qh) | (0b01 << 1);
        log_debug!("add_qh_to_interrupt({qh:?}): addr={addr:#x}");
        for s in (0 .. 1024).step_by(period).map(|s| best_ofs + s) {
            // SAFE: Lock is held
            let (prev, n) = unsafe { self.intr_find_insert_point(pq[s], period) };
            assert!(n == 1 || n == next);
            if let Some(prev) = prev {
                // SAFE: Lock is held, a hardware read doesn't matter
                unsafe { self.intr_set_next(prev, addr) };
            }
            else {
                pq[s] = addr;
            }
        }

        IntHandle {
            qh,
        }
    }

    pub(crate) fn remove_qh_from_interrupt(&self, h: IntHandle)
    {
        // Get the period
        // Determine what slots are used
        // Visit all slots and remove this header
        todo!("remove_qh_from_interrupt");
    }

    /// Wait for an interrupt to complete
    pub(crate) async fn wait_for_interrupt(&self, h: &mut IntHandle, mut next_td: crate::desc_pools::TdHandle) -> crate::desc_pools::TdHandle
    {
        log_debug!("wait_for_interrupt({:?}): next {:?}", h.qh, next_td);
        self.td_pool.get_data_mut(&mut next_td).token |= hw_structs::QTD_TOKEN_STS_ACTIVE | hw_structs::QTD_TOKEN_IOC;
        // TODO: I'd like to append `next_td` to the currently queued one before waiting - but that's questionably safe
        //self.td_pool.set_next( self.qh_pool.get_first_td(&mut h.qh), next_td );
        // Instead, assume that the post-wait code runs soon enough that there isn't much jitter

        self.qh_pool.wait(&mut h.qh).await;
        let rv = self.qh_pool.clear_td(&mut h.qh).expect("Interrupt queue head didn't already have an allocated TD");
        self.qh_pool.assign_td(&mut h.qh, &self.td_pool, next_td);
        log_debug!("wait_for_interrupt({:?}): return {:?}", h.qh, rv);
        rv
    }

    /// Reads the entry pointed to by `queue_ent` and returns it's hlink value and the interrupt period
    unsafe fn intr_get_next_and_period(&self, queue_ent: u32) -> (u32, usize) {
        match (queue_ent >> 1) & 3
        {
        0b00 => todo!("iTD"),
        0b01 => self.qh_pool.get_next_and_period(queue_ent & !0x1F),
        0b10 => todo!("siTD"),
        0b11 => todo!("FSTD"),
        _ => unreachable!(),
        }
    }

    /// Set the `hlink` pointer of a queue entry
    unsafe fn intr_set_next(&self, queue_ent: u32, next: u32) {
        match (queue_ent >> 1) & 3
        {
        0b00 => todo!("iTD"),
        0b01 => self.qh_pool.set_next(queue_ent & !0x1F, next),
        0b10 => todo!("siTD"),
        0b11 => todo!("FSTD"),
        _ => unreachable!(),
        }
    }

    unsafe fn intr_find_insert_point(&self, mut queue_ent: u32, period: usize) -> (Option<u32>, u32) {
        let mut prev = None;
        loop
        {
            if queue_ent & 1 != 0 {
                break (None, 1);
            }

            // Load the entry
            let (next, p) = self.intr_get_next_and_period(queue_ent);
            if p <= period {
                break (prev, queue_ent);
            }
            prev = Some(queue_ent);
            queue_ent = next;
        }
    }

    unsafe fn intr_calculate_load(&self, mut queue_ent: u32) -> usize {
        let mut load = 0;
        loop
        {
            if queue_ent & 1 != 0 {
                break load;
            }

            // Load the entry
            let (next, p) = self.intr_get_next_and_period(queue_ent);
            if p < usize::max_value() {
                load += 1;
            }
            queue_ent = next;
        }
    }
}

pub struct IntHandle
{
    qh: crate::desc_pools::QhHandle,
}
