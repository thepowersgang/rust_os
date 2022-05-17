// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/hpet.rs
// - x86 High Precision Event Timer
#[allow(unused_imports)]
use crate::prelude::*;
use crate::arch::amd64::acpi::AddressSpaceID;

module_define!{HPET, [APIC, ACPI], init}

struct HPET
{
	mapping_handle: crate::memory::virt::AllocHandle,
	#[allow(dead_code)]
	irq_handle: crate::arch::amd64::hw::apic::IRQHandle,
	period: u64,
}

#[repr(C,packed)]
struct ACPI_HPET
{
	hw_rev_id: u8,
	flags: u8,
	pci_vendor: u16,
	addr: crate::arch::amd64::acpi::GAS,
	hpet_num: u8,
	mintick: [u8; 2],	// 16-bit word
	page_protection: u8,
}

#[repr(C)]
#[derive(Copy,Clone)]
enum HPETReg
{
	// NOTE: `2*` accounts for the fact that all regs are 64-bit (8 bytes) but the main regs are spaced by 16 bytes
	CapsID  = 2*0x0,
	Config  = 2*0x1,
	ISR     = 2*0x2,
	MainCtr = 2*0xF,
	Timer0  = 2*0x10,
}
#[repr(C)]
#[derive(Copy,Clone)]
enum HPETCompReg
{
	ConfigCaps  = 0x0,
	Value       = 0x1,
	#[allow(dead_code)]
	FsbIntRoute = 0x2,
}

static S_INSTANCE: crate::lib::LazyStatic<HPET> = lazystatic_init!();
static S_LOCK: crate::sync::Spinlock<()> = crate::sync::Spinlock::new( () );

/// Reutrns the current system timestamp, in miliseconds since an arbitary point (usually power-on)
pub fn get_timestamp() -> u64
{
	if S_INSTANCE.ls_is_valid() {
		S_INSTANCE.current() / S_INSTANCE.ticks_per_ms()
	}
	else {
		0
	}
}
pub fn request_tick(target_time: u64)
{
	if S_INSTANCE.ls_is_valid() {
		let _lh = S_LOCK.lock();

		let new_target = target_time * S_INSTANCE.ticks_per_ms();
		
		let cur_target = S_INSTANCE.get_target(0);
		let cur_value = S_INSTANCE.current();
		if new_target < cur_value {
			// The new target is in the past!
			log_warning!("Requesting a tick in the past {:#x} < {:#x}", new_target, cur_value);
			crate::irqs::timer_trigger();
		}
		// If the current target is in the past (i.e. it's already triggered), or the new target is earlier than the current
		else if cur_value > cur_target || new_target < cur_target {
			// Update target to the new target
			log_debug!("Registering oneshot(={new_target:#x}): cur={cur_value:#x}");
			S_INSTANCE.oneshot(0, new_target);
		}
		else {
			// There's a pre-existing target time before this
			log_debug!("Pre-existing oneshot({cur_target:#x} < {new_target:#x}): cur={cur_value:#x}");
		}
	}
	else {
		log_error!("Tick requested before HPET initialised");
	}
}

fn init()
{
	log_trace!("init()");
	let hpet = match crate::arch::amd64::acpi::find::<ACPI_HPET>("HPET", 0)
		{
		None => {
			log_error!("No HPET in ACPI, no timing avaliable");
			return ;
			},
		Some(v) => v,
		};

	let info = hpet.data();
	assert!(info.addr.asid == AddressSpaceID::Memory as u8);
	assert!(info.addr.address % crate::PAGE_SIZE as u64 == 0, "Address {:#x} not page aligned", { info.addr.address });
	// Assume SAFE: Shouldn't be sharing paddrs
	let mapping = unsafe { crate::memory::virt::map_hw_rw(info.addr.address, 1, "HPET").unwrap() };

	// HACK! Disable the PIT
	// - This should really be done by the ACPI code (after it determines the PIT exists)
	// SAFE: Nothing else attacks the PIT
	unsafe {
		super::pit::disable();
	}

	// SAFE: 'init' is called in a single-threaded context
	let inst = unsafe {
		S_INSTANCE.prep(|| HPET::new(mapping));
		S_INSTANCE.ls_unsafe_mut().bind_irq();
		&*S_INSTANCE
		};
	
	inst.oneshot(0, inst.current() + 100*1000 );
	let _ = inst;
}

impl HPET
{
	pub fn new(mapping: crate::memory::virt::AllocHandle) -> HPET
	{
		let mut rv = HPET {
			mapping_handle: mapping,
			irq_handle: Default::default(),
			period: 1,
			};
		// Enable
		rv.write_reg(HPETReg::Config as usize, rv.read_reg(HPETReg::Config as usize) | (1 << 0));
		let caps_id = rv.read_reg(HPETReg::CapsID as usize);
		log_debug!("Capabilities/ID: {caps_id:#x}");
		rv.period = caps_id >> 32;
		for comparitor in 0..rv.num_comparitors() {
			let comp_reg = HPETReg::Timer0 as usize + comparitor*4;
			let v = rv.read_reg(comp_reg);
			log_debug!("Comp {comparitor}: {v:#x}");
		}
		rv
	}
	pub fn bind_irq(&mut self)
	{
		self.irq_handle = crate::arch::amd64::hw::apic::register_irq(2, HPET::irq, self as *mut _ as *const _).unwrap();
	}
	pub fn ticks_per_ms(&self) -> u64
	{
		// period = fempto (15) seconds per tick
		// Want ticks per ms
		// 
		1000*1000*1000*1000 / self.period
	}
	
	fn irq(sp: *const ())
	{
		// SAFE: Pointer associated should be an instance of HPET
		let s = unsafe{ &*(sp as *const HPET) };
		let isr = s.read_reg(HPETReg::ISR as usize);
		s.write_reg(HPETReg::ISR as usize, isr);

		if isr & 1 != 0
		{
			crate::irqs::timer_trigger();
		}
	}
	
	fn read_reg(&self, reg: usize) -> u64 {
		// SAFE: Hardware access, implicitly atomic on x86
		unsafe {
			let p: *const _ = & (*self.regs())[reg];
			::core::intrinsics::volatile_load( p )
			//let v1 = ::core::intrinsics::volatile_load( (p as *const u32).offset(0) );
			//let v2 = ::core::intrinsics::volatile_load( (p as *const u32).offset(1) );
			//v1 as u64 | ((v2 as u64) << 32)
		}
	}
	fn write_reg(&self, reg: usize, val: u64) {
		// SAFE: Hardware access, implicitly atomic on x86
		unsafe {
			::core::intrinsics::volatile_store( &mut (*self.regs())[reg], val )
			//let p: *mut _ = &mut (*self.regs())[reg];
			//::core::intrinsics::volatile_store( (p as *mut u32).offset(0), (val >>  0) as u32 );
			//::core::intrinsics::volatile_store( (p as *mut u32).offset(1), (val >> 32) as u32 );
		}
	}
	fn regs(&self) -> *mut [u64; 0x100] {
		// SAFE: Coerces to raw pointer instantly
		unsafe { self.mapping_handle.as_int_mut(0) }
	}
	fn num_comparitors(&self) -> usize {
		((self.read_reg(HPETReg::CapsID as usize) >> 8) & 0x1F) as usize + 1
	}
	
	fn current(&self) -> u64 {
		self.read_reg(HPETReg::MainCtr as usize)
	}
	fn comparitor(&self, idx: usize) -> Comparitor<'_> {
		assert!(idx < self.num_comparitors());
		Comparitor { parent: self, idx }
	}
	fn oneshot(&self, comparitor: usize, value: u64) {
		self.comparitor(comparitor).set_oneshot(value);
	}
	fn get_target(&self, comparitor: usize) -> u64 {
		self.comparitor(comparitor).get_target()
	}
}
struct Comparitor<'a>
{
	parent: &'a HPET,
	idx: usize,
}
impl Comparitor<'_>
{
	#[track_caller]
	fn reg(&self, index: HPETCompReg) -> usize {
		assert!( (index as usize) < 4 );
		HPETReg::Timer0 as usize + self.idx * 4 + index as usize
	}
	#[track_caller]
	fn write_reg(&self, index: HPETCompReg, val: u64) {
		self.parent.write_reg(self.reg(index), val);
	}
	#[track_caller]
	fn read_reg(&self, index: HPETCompReg) -> u64 {
		self.parent.read_reg( self.reg(index) )
	}

	fn set_oneshot(&self, target: u64) {
		log_debug!("[{}] set_oneshot({:#x})", self.idx, target);
		// Set comparitor value
		self.write_reg(HPETCompReg::Value, target);
		// HACK: Wire to APIC interrupt 2
		// IRQ2, Interrups Enabled, Level Triggered
		self.write_reg(HPETCompReg::ConfigCaps, (2 << 9)|(1<<2)|(1<<1));
	}
	fn get_target(&self) -> u64 {
		self.read_reg(HPETCompReg::Value)
	}
}

// vim: ft=rust

