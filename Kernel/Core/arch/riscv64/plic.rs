use ::core::sync::atomic::{Ordering, AtomicUsize, AtomicU32};

pub struct PlicInstance(AtomicUsize);

const TARGET_CONTEXT: usize = 1;	// 0 = HART0:M, 1 = HART0:S
impl PlicInstance
{
	pub const fn new_uninit() -> PlicInstance {
		PlicInstance(AtomicUsize::new(0))
	}
	pub fn init(&self, ah: crate::memory::virt::MmioHandle) {
		// SAFE: Access is currently unique
		let ptr: *mut () = unsafe { ah.as_int_mut(0) };
		match self.0.compare_exchange(0, ptr as usize, Ordering::SeqCst, Ordering::Relaxed)
		{
		Ok(_) => {
			// Allow any IRQ
			self.get_target_prio_reg(TARGET_CONTEXT).store(0, Ordering::Relaxed);
			::core::mem::forget(ah);
			},
		Err(other) => {
			log_error!("Multiple PLICs registered? - {:#x} and {:#x}", other, ptr as usize);
			}
		}
	}
	pub fn is_init(&self)->bool {
		self.0.load(Ordering::SeqCst) != 0
	}

	/// Enable the specified interrupt
	pub fn set_enable(&self, index: usize, enable: bool) {
		let ofs = index / 32;
		let mask = 1 << (index % 32);
		let slot = &self.get_enable_bits(TARGET_CONTEXT)[ofs];
		if enable {
			log_debug!("Enabling {}", index);
			slot.fetch_or(mask, Ordering::Relaxed);
			self.get_prio_map()[index].store(1, Ordering::Relaxed);
		}
		else {
			log_debug!("Disabling {}", index);
			self.get_prio_map()[index].store(0, Ordering::Relaxed);
			slot.fetch_and(!mask, Ordering::Relaxed);
		}
	}
	/// Loop running claim+complete until zero is returned
	pub fn claim_complete_cycle(&self, mut cb: impl FnMut(usize)) {
		let r = self.get_claim_reg(TARGET_CONTEXT);
		loop
		{
			let v = r.load(Ordering::Relaxed);
			log_trace!("v = {}", v);
			if v == 0 {
				break;
			}
			cb(v as usize);
			r.store(v, Ordering::Relaxed);
		}
	}

	fn get_ref<T: crate::lib::POD>(&self, ofs: usize) -> &T {
		assert!(ofs + ::core::mem::size_of::<T>() < 0x4_000_000);
		let base = self.0.load(Ordering::SeqCst);
		assert!(base != 0, "Using unintialised PlicInstance");
		// SAFE: Once non-zero, this pointer is always valid. Range checked above
		unsafe { &*( (base + ofs) as *const T ) }
	}

	fn get_prio_map(&self) -> &[AtomicU32; 1024] {
		self.get_ref(0)
	}
	//fn get_pending_bits(&self) -> &[AtomicU32; 1024/32] {
	//	self.get_ref(0x1000)
	//}
	fn get_enable_bits(&self, context: usize) -> &[AtomicU32; 1024/32] {
		self.get_ref(0x002000 + context * (1024/8))
	}
	fn get_target_prio_reg(&self, context: usize) -> &AtomicU32 {
		assert!(context < 15872);
		self.get_ref(0x200000 + context * 0x1000)
	}
	fn get_claim_reg(&self, context: usize) -> &AtomicU32 {
		assert!(context < 15872);
		self.get_ref(0x200004 + context * 0x1000)
	}
}
