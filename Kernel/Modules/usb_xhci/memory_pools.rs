
use ::core::sync::atomic::{Ordering,AtomicU64};

const N_PAGES: usize = 4;
const SLOT_SIZE: usize = 0x20;
// 0x1000/0x20 = 0x80 = 128 blocks (endpoints) per page
const N_SLOTS_PER_PAGE: usize = ::kernel::PAGE_SIZE / SLOT_SIZE;
const POOL0_SLOTS: usize = 256*8 / SLOT_SIZE;

/// A pool of hardware-accessible pages that can be allocated in chunks of 32 bytes
pub(crate) struct MemoryPools
{
    /// Device context pointers, only the first half is used (256 * 8 = 0x800). Second half is pool zero for the device contexts
    dcbaa: ::kernel::memory::virt::ArrayHandle<AtomicU64>,
    /// General pools of items
    pools: [Option<::kernel::memory::virt::ArrayHandle<[u32; SLOT_SIZE/4]>>; N_PAGES],

    used: ::kernel::sync::Spinlock<Used>,
}
#[derive(Default)]
struct Used {
    pool0: [ u32; 2 ],
    pools: [ [u32; N_SLOTS_PER_PAGE/32]; 4],
}

impl MemoryPools
{
    pub(crate) fn new(dcbaa: ::kernel::memory::virt::ArrayHandle<u64>) -> MemoryPools {
        MemoryPools {
            // SAFE: Same repr, and I'm lazy
            dcbaa: unsafe { ::core::mem::transmute(dcbaa) },
            pools: [None, None, None, None],
            used: Default::default(),
        }
    }
    /// Update an entry in the DCBA
    pub(crate) unsafe fn set_dcba(&self, index: u8, handle: u64) {
        self.dcbaa[index as usize].store(handle, Ordering::Relaxed);
    }

    pub(crate) fn alloc(&self, n_blocks: u8) -> Option<PoolHandle> {
        let mut used = self.used.lock();
        // Look for a block with sufficient free bits
        // 1. Try pool 0
        if let Some(rel_slot) = find_and_set(&mut used.pool0, n_blocks) {
            return Some(PoolHandle {
                pool: 0,
                index: POOL0_SLOTS as u8 + rel_slot as u8,
                count: n_blocks,
                });
        }
        for pool in 0 .. N_PAGES {
            if let Some(rel_slot) = find_and_set(&mut used.pools[pool], n_blocks) {
                return Some(PoolHandle {
                    pool: 1 + pool as u8,
                    index: rel_slot as u8,
                    count: n_blocks,
                    });
            }
        }
        None
    }
    pub(crate) fn release(&self, handle: PoolHandle) {
        let mut used = self.used.lock();
        if handle.pool == 0 {
            clear_bits(&mut used.pool0, handle.index - POOL0_SLOTS as u8, handle.count);
        }
        else {
            clear_bits(&mut used.pools[handle.pool as usize - 1], handle.index, handle.count);
        }
        ::core::mem::forget(handle);    // The drop impl of `PoolHadle` emits an error, so suppress that
    }

    pub(crate) fn get(&self, handle: &PoolHandle) -> *const [u32; 0x20 / 4] {
        if handle.pool == 0 {
            &self.dcbaa[handle.index as usize * SLOT_SIZE / 8] as *const _ as *const _
        }
        else {
            &self.pools[handle.pool as usize - 1].as_ref().unwrap()[handle.index as usize]
        }
    }

    pub(crate) fn get_phys(&self, handle: &PoolHandle) -> u64 {
        ::kernel::memory::virt::get_phys(self.get(handle)) as u64
    }
}


fn find_and_set(data: &mut [u32], n_blocks: u8) -> Option<usize> {
    'outer: for ofs in 0 .. data.len()*32 {
        for i in 0 .. n_blocks {
            let idx = ofs + i as usize;
            if data[idx / 32] & (1 << (idx%32)) != 0 {
                continue 'outer;
            }
        }
        // Found a slot!
        for i in 0 .. n_blocks {
            let idx = ofs + i as usize;
            data[idx / 32] |= 1 << (idx%32);
        }
        return Some(ofs);
    }
    None
}
fn clear_bits(data: &mut [u32], ofs: u8, n_blocks: u8) -> bool {
    for i in 0 .. n_blocks {
        let idx = (ofs + i) as usize;
        if data[idx / 32] & (1 << (idx%32)) == 0 {
            return false;
        }
    }
    for i in 0 .. n_blocks {
        let idx = (ofs + i) as usize;
        data[idx / 32] &= !(1 << (idx%32));
    }
    true
}

pub(crate) struct PoolHandle
{
    /// Pool index
    pool: u8,
    /// Index into the pool (one page has 128 possible 32-byte slots)
    index: u8,
    /// Number of 32-bye slots used
    count: u8,
}
impl PoolHandle {
    pub fn len(&self) -> usize {
        self.count as usize
    }
}
impl ::core::ops::Drop for PoolHandle {
    fn drop(&mut self) {
        log_error!("Dropped PoolHandle({},{}+{})", self.pool, self.index, self.count);
    }
}