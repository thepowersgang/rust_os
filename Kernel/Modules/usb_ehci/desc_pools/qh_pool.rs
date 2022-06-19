//!
//! 
//! 
use ::core::cell::UnsafeCell;
use ::core::convert::TryInto;
use crate::hw_structs;
use super::UnsafeArrayHandle;

/// Queue head pool
pub struct QhPool {
    alloc: UnsafeArrayHandle<hw_structs::QueueHead>,
    sem: ::kernel::sync::Semaphore,
    alloced: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
    released: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
    meta: [UnsafeCell<QhMeta>; Self::COUNT],
    waiters: [::kernel::futures::flag::SingleFlag; Self::COUNT],
}
unsafe impl Sync for QhPool {}
unsafe impl Send for QhPool {}
impl QhPool {
    const COUNT: usize = ::kernel::PAGE_SIZE / ::core::mem::size_of::<hw_structs::QueueHead>();

    pub fn new() -> Result<Self,&'static str> {
        Ok(QhPool {
            alloc: UnsafeArrayHandle::new( ::kernel::memory::virt::alloc_dma(32, 1, module_path!())? ),
            sem: ::kernel::sync::Semaphore::new(Self::COUNT as isize, Self::COUNT as isize),
            alloced: ::kernel::sync::Spinlock::new( [0; (Self::COUNT + 7) / 8] ),
            released: ::kernel::sync::Spinlock::new( [0; (Self::COUNT + 7) / 8] ),
            meta: [(); Self::COUNT].map(|_| UnsafeCell::new(QhMeta { td: None })),
            waiters: [(); Self::COUNT].map(|_| Default::default()),
        })
    }
    pub fn alloc(&self, endpoint_id: u32, endpoint_ext: u32) -> QhHandle {
        let mut rv = self.alloc_raw(crate::hw_structs::QueueHead {
            hlink: 1,
            endpoint: endpoint_id,
            endpoint_ext: endpoint_ext,
            current_td: 0,
            overlay_link: 0,
            overlay_link2: 0,
            overlay_token: 0,
            overlay_pages: [0; 5]
            });
        self.get_meta_mut(&mut rv).td = None;
        rv
    }
    pub fn alloc_raw(&self, v: hw_structs::QueueHead) -> QhHandle {
        self.sem.acquire();
        let mut lh = self.alloced.lock();
        match super::set_first_zero_bit(&mut lh[..])
        {
        Some(i) => {
            let mut rv = QhHandle(i);
            *self.get_data_mut(&mut rv) = v;
            self.waiters[rv.0].reset();
            rv
            },
        None => panic!("All slots are used, but semaphore was acquired"),
        }
    }
    pub fn release(&self, handle: QhHandle) {
        log_debug!("QhPool::release({:?})", handle);
        let idx = handle.0;
        ::core::mem::forget(handle);
        let mut lh = self.released.lock();
        lh[idx / 8] |= 1 << (idx % 8);
    }

    /// Assigns a TD to the queue, and starts it executing
    pub fn assign_td(&self, handle: &mut QhHandle, td_pool: &super::TdPool, first_td: super::TdHandle) {
        let d = self.get_data_mut(handle);
        //let s = td_pool.get_data(&first_td);
        d.current_td = td_pool.get_phys(&first_td)/*| crate::hw_structs::QH*/;
        d.overlay_link = d.current_td;
        self.get_meta_mut(handle).td = Some(first_td);
        d.overlay_token = 0;    // Clear all data to start execution of the queue
    }
    pub fn clear_td(&self, handle: &mut QhHandle) -> Option<super::TdHandle> {
        self.get_data_mut(handle).current_td = 0;
        self.get_meta_mut(handle).td.take()
    }

    fn get_idx_from_phys(&self, addr: u32) -> usize {
        let phys0: u32 = self.alloc.get_phys(0).try_into().unwrap();
        assert!(addr >= phys0);
        let idx = (addr - phys0) / ::core::mem::size_of::<hw_structs::QueueHead>() as u32;
        let idx = idx as usize;
        assert!(idx < Self::COUNT);
        idx
    }

    pub fn get_phys(&self, h: &QhHandle) -> u32 {
        self.alloc.get_phys(h.0).try_into().unwrap()
    }
    pub fn get_data(&'_ self, h: &'_ QhHandle) -> &'_ hw_structs::QueueHead {
        // SAFE: Shared access to the handle implies shared access to the data
        unsafe { self.alloc.get(h.0) }
    }
    pub fn get_data_mut(&'_ self, h: &'_ mut QhHandle) -> &'_ mut hw_structs::QueueHead {
        // SAFE: The handle is owned
        unsafe { self.alloc.get_mut(h.0) }
    }
    /*pub*/ fn get_meta_mut(&'_ self, h: &'_ mut QhHandle) -> &'_ mut QhMeta {
        // SAFE: Mutable access to the handle implies mutable access to the data
        unsafe { &mut *self.meta[h.0].get() }
    }


    /// UNSAFE: Only call this once the controller is no longer accessing any released entries
    /// (i.e. the queue is stopped, or the queue has been advanced)
    pub unsafe fn trigger_gc(&self) {
        log_trace!("QhPool::trigger_gc");
        // Iterate all entries, look for one marked as released
        let mut lh_release = self.released.lock();
        let mut lh_alloc = self.alloced.lock();
        for idx in 0 .. Self::COUNT {
            if super::get_and_clear_bit(&mut lh_release[..], idx) {
                assert!(super::get_and_clear_bit(&mut lh_alloc[..], idx));
                self.sem.release();
            }
        }
    }
    /// Remove a QH from a list
    /// 
    /// UNSAFE: Caller must ensure that the entry is on the queue/loop started by `root`
    pub unsafe fn remove_from_list(&self, root: &mut QhHandle, ent: &QhHandle) {
        let mut cur_idx = root.0;
        loop {
            let hlink = self.alloc.get(cur_idx).hlink;
            if hlink == 0 {
                // Not found?
                return ;
            }
            let next = self.get_idx_from_phys(hlink & !0xF);
            if next == ent.0 {
                // Found it!
                log_debug!("QhPool::remove_from_list: Stich {cur_idx} to {next}, removing {ent}", ent=ent.0);
                self.alloc.get_mut(cur_idx).hlink = self.alloc.get(next).hlink;
                return ;
            }
            cur_idx = next;
            if cur_idx == root.0 {
                // Uh-oh, we've looped. Error?
                return ;
            }
        }
    }

    /// Iterate a queue started by `first` and check for completed queues
    /// 
    /// UNSAFE: Caller must ensure that the 
    pub unsafe fn check_completion(&self, first: &QhHandle) {
        let mut cur_idx = first.0;
        loop {
            let cur = self.alloc.get(cur_idx);
            let hlink = cur.hlink;
            if hlink == 0 {
                // Not found?
                return ;
            }
            let next = self.get_idx_from_phys(hlink & !0xF);

            if cur.overlay_token & hw_structs::QTD_TOKEN_STS_ACTIVE == 0 {
                let meta = &*self.meta[cur_idx].get();
                // Inactive, wake it?
                if meta.td.is_some() {
                    self.waiters[cur_idx].trigger();
                }
            }

            cur_idx = next;
            if cur_idx == first.0 {
                // Uh-oh, we've looped. Error?
                return ;
            }
        }
    }

    /// Async wait for the QH to be removed from the async queue
    pub async fn wait(&self, h: &mut QhHandle) {
        self.waiters[h.0].wait().await
    }
}
#[derive(Debug)]
pub struct QhHandle(usize);
impl ::core::ops::Drop for QhHandle
{
    fn drop(&mut self) {
        log_error!("BUG: {:?} dropped, should be released back to the pool", self);
    }
}
struct QhMeta {
    /// The first item in the linked list of owned TDs
    td: Option<super::TdHandle>,
}