//!
//! 
//! 
use ::core::cell::UnsafeCell;
use ::core::convert::TryInto;
use crate::hw_structs;
use super::UnsafeArrayHandle;

/// Transfer descriptor pool
pub struct TdPool {
    alloc: UnsafeArrayHandle<hw_structs::TransferDesc>,
    sem: ::kernel::sync::Semaphore,
    alloced: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
    meta: [UnsafeCell<TdMeta>; Self::COUNT],
}
unsafe impl Send for TdPool {}
unsafe impl Sync for TdPool {}
impl TdPool {
    const COUNT: usize = ::kernel::PAGE_SIZE / ::core::mem::size_of::<hw_structs::TransferDesc>();

    pub fn new() -> Result<Self,&'static str> {
        Ok(TdPool {
            alloc: UnsafeArrayHandle::new( ::kernel::memory::virt::alloc_dma(32, 1, module_path!())? ),
            sem: ::kernel::sync::Semaphore::new(Self::COUNT as isize, Self::COUNT as isize),
            alloced: ::kernel::sync::Spinlock::new( [0; (Self::COUNT + 7) / 8] ),
            meta: [(); Self::COUNT].map(|_| UnsafeCell::new(TdMeta { next: None })),
        })
    }

    /// UNSAFE: This will record the pointer from `data` in the buffer, and may write to it (depending on the packet type)
    /// Callers must ensure that `data` is valid until the hardware is done with it
    pub unsafe fn alloc(&self, packet_id: hw_structs::Pid, data: &[u8], next: Option<TdHandle>) -> TdHandle {
        assert!(data.len() < ::kernel::PAGE_SIZE);
        let phys0 = ::kernel::memory::virt::get_phys(data.as_ptr());
        let phys0_tail = (phys0 - phys0 % ::kernel::PAGE_SIZE as u64) as usize;
        let phys1 = if data.len() < phys0_tail {
                0
            } else {
                ::kernel::memory::virt::get_phys(data[phys0_tail..].as_ptr())
            };
        let (phys0, phys1) = match (phys0.try_into(), phys1.try_into())
            {
            (Ok(a),Ok(b)) => (a,b),
            _ => todo!("TdPool::alloc: Handle 64-bit physical addresses"),
            };
        self.alloc_raw(hw_structs::TransferDesc {
            link: if let Some(ref next) = next { self.get_phys(&next) } else { 1 },
            link2: 1,
            token: (packet_id as u32) << 8 | (data.len() as u32) << 16,
            pages: [
                phys0,
                phys1,
                0,0,0,
                ]
            }, next)
    }
    fn alloc_raw(&self, v: hw_structs::TransferDesc, next: Option<TdHandle>) -> TdHandle {
        self.sem.acquire();
        match super::set_first_zero_bit(&mut self.alloced.lock()[..])
        {
        Some(i) => {
            let mut rv = TdHandle(i);
            log_debug!("TdPool::alloc_raw(next={:?}): {:?}", next, rv);
            self.get_meta_mut(&mut rv).next = next;
            *self.get_data_mut(&mut rv) = v;
            rv
            },
        None => panic!("All slots are used, but semaphore was acquired"),
        }
    }
    pub fn release(&self, mut handle: TdHandle) -> Option<TdHandle> {
        let rv = self.get_meta_mut(&mut handle).next.take();
        log_debug!("TdPool::release({:?}): next={:?}", handle, rv);
        let idx = handle.0;
        ::core::mem::forget(handle);
        let mut lh = self.alloced.lock();
        if !super::get_and_clear_bit(&mut lh[..], idx) {
            panic!("Releasing an unused handle {}", idx);
        }
        self.sem.release();
        rv
    }
    pub fn get_phys(&self, h: &TdHandle) -> u32 {
        self.alloc.get_phys(h.0).try_into().unwrap()
    }
    pub fn get_data(&self, h: &TdHandle) -> &hw_structs::TransferDesc {
        // SAFE: Shared access to the handle implies shared access to the data
        unsafe { self.alloc.get(h.0) }
    }
    pub fn get_data_mut(&self, h: &mut TdHandle) -> &mut hw_structs::TransferDesc {
        // SAFE: Mutable access to the handle implies mutable access to the data
        unsafe { self.alloc.get_mut(h.0) }
    }
    /*pub*/ fn get_meta_mut(&self, h: &mut TdHandle) -> &mut TdMeta {
        // SAFE: Mutable access to the handle implies mutable access to the data
        unsafe { &mut *self.meta[h.0].get() }
    }

    /// Iterate through the chain of descriptors starting from `root`
    pub fn iter_chain_mut(&self, root: &mut TdHandle, mut cb: impl FnMut(&mut hw_structs::TransferDesc/* , &mut TdMeta*/)) {
        let mut cur_idx = root.0;
        loop {
            let (data, _meta) = unsafe {
                (self.alloc.get_mut(cur_idx), &mut *self.meta[cur_idx].get())
            };
            let link = data.link;
            cb(data/*, meta*/);
            assert_eq!(data.link, link, "Link changed in `iter_chain_mut` - don't do that");
            if link & 1 == 1 {
                break;
            }

            cur_idx = self.get_idx_from_phys(link & !0xF);
        }
    }

    fn get_idx_from_phys(&self, addr: u32) -> usize {
        let phys0: u32 = self.alloc.get_phys(0).try_into().unwrap();
        assert!(addr >= phys0);
        let idx = (addr - phys0) / ::core::mem::size_of::<hw_structs::TransferDesc>() as u32;
        let idx = idx as usize;
        assert!(idx < Self::COUNT);
        idx
    }
}
#[derive(Debug)]
pub struct TdHandle(usize);
impl Drop for TdHandle {
    fn drop(&mut self) {
        log_error!("BUG: {:?} dropped, should be released back to the pool", self);
    }
}
#[derive(Default)]
struct TdMeta {
    next: Option<TdHandle>,
}
