//!
use ::core::cell::UnsafeCell;
use ::kernel::memory::virt::ArrayHandle;
use crate::hw_structs;

fn set_first_zero_bit(arr: &mut [u8]) -> Option<usize> {
    for (i,s) in arr.iter_mut().enumerate() {
        if *s != 0xFF {
            let j = s.trailing_ones() as usize;
            *s |= 1 << j;
            return Some(i * 8 + j);
        }
    }
    None
}
fn get_and_clear_bit(arr: &mut [u8], idx: usize) -> bool {
    let bit = 1 << (idx % 8);
    let s = &mut arr[idx / 8];
    let rv = *s & bit != 0;
    *s &= !bit;
    rv
}
struct UnsafeArrayHandle<T> {
    inner: ::kernel::memory::virt::AllocHandle,
    pd: ::core::marker::PhantomData<::core::cell::UnsafeCell<T>>,
}
unsafe impl<T: Sync> Sync for UnsafeArrayHandle<T> {}
unsafe impl<T: Send> Send for UnsafeArrayHandle<T> {}
impl<T: ::kernel::lib::POD> UnsafeArrayHandle<T> {
    fn new(inner: ::kernel::memory::virt::AllocHandle) -> Self {
        Self { inner, pd: ::core::marker::PhantomData }
    }
    fn get_phys(&self, idx: usize) -> ::kernel::memory::PAddr {
        ::kernel::memory::virt::get_phys::<T>( self.inner.as_ref(idx * ::core::mem::size_of::<T>()) )
    }
    unsafe fn get(&self, idx: usize) -> &T {
        self.inner.as_ref(idx * ::core::mem::size_of::<T>())
    }
    unsafe fn get_mut(&self, idx: usize) -> &mut T {
        self.inner.as_int_mut(idx * ::core::mem::size_of::<T>())
    }
}

pub struct TdPool {
    alloc: UnsafeArrayHandle<hw_structs::TransferDesc>,
    sem: ::kernel::sync::Semaphore,
    alloced: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
}
impl TdPool {
    const COUNT: usize = ::kernel::PAGE_SIZE / ::core::mem::size_of::<hw_structs::TransferDesc>();

    pub fn new() -> Result<Self,&'static str> {
        Ok(TdPool {
            alloc: UnsafeArrayHandle::new( ::kernel::memory::virt::alloc_dma(32, 1, module_path!())? ),
            sem: ::kernel::sync::Semaphore::new(Self::COUNT as isize, Self::COUNT as isize),
            alloced: ::kernel::sync::Spinlock::new( [0; (Self::COUNT + 7) / 8] ),
        })
    }

    pub fn alloc(&self, v: hw_structs::TransferDesc) -> TdHandle {
        self.sem.acquire();
        let mut lh = self.alloced.lock();
        match set_first_zero_bit(&mut lh[..])
        {
        Some(i) => {
            let mut rv = TdHandle(i);
            *self.get_data_mut(&mut rv) = v;
            rv
            },
        None => panic!("All slots are used, but semaphore was acquired"),
        }
    }
    pub fn release(&self, handle: TdHandle) {
        let idx = handle.0;
        ::core::mem::forget(handle);
        let mut lh = self.alloced.lock();
        if !get_and_clear_bit(&mut lh[..], idx) {
            panic!("Releasing an unused handle {}", idx);
        }
        self.sem.release();
    }
    pub fn get_phys(&self, h: &TdHandle) -> u32 {
        use ::core::convert::TryInto;
        self.alloc.get_phys(h.0).try_into().unwrap()
    }
    pub fn get_data_mut(&self, h: &mut TdHandle) -> &mut hw_structs::TransferDesc {
        unsafe { self.alloc.get_mut(h.0) }
    }
}
pub struct TdHandle(usize);
impl Drop for TdHandle {
    fn drop(&mut self) {
    }
}


pub struct QhPool {
    alloc: UnsafeArrayHandle<hw_structs::QueueHead>,
    sem: ::kernel::sync::Semaphore,
    alloced: ::kernel::sync::Spinlock<[u8; (Self::COUNT + 7) / 8]>,
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
        })
    }
    pub fn alloc(&self, v: hw_structs::QueueHead) -> QhHandle {
        self.sem.acquire();
        let mut lh = self.alloced.lock();
        match set_first_zero_bit(&mut lh[..])
        {
        Some(i) => {
            let mut rv = QhHandle(i);
            *self.get_data_mut(&mut rv) = v;
            rv
            },
        None => panic!("All slots are used, but semaphore was acquired"),
        }
    }
    pub fn release(&self, handle: QhHandle) {
        let idx = handle.0;
        ::core::mem::forget(handle);
        let mut lh = self.alloced.lock();
        if !get_and_clear_bit(&mut lh[..], idx) {
            panic!("Releasing an unused handle {}", idx);
        }
        self.sem.release();
    }
    pub fn get_phys(&self, h: &QhHandle) -> u32 {
        use ::core::convert::TryInto;
        self.alloc.get_phys(h.0).try_into().unwrap()
    }
    pub fn get_data_mut(&self, h: &mut QhHandle) -> &mut hw_structs::QueueHead {
        // SAFE: The handle is owned
        unsafe { self.alloc.get_mut(h.0) }
    }
}
pub struct QhHandle(usize);
//pub struct QueueHeadMeta {
//}