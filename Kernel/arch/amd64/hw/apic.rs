// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/apic.rs
// - x86 APIC (Advanced Programmable Interrupt Controller) driver
use _common::*;
use core::ptr::RawPtr;

module_define!(APIC, [ACPI], init)

#[repr(C,packed)]
struct ACPI_MADT
{
	local_controller_addr: u32,
	flags: u32,
	end: [u8,..0],
}
#[repr(C,packed)]
struct MADT_DevHeader
{
	dev_type: u8,
	rec_len: u8,
}
#[repr(C,packed)]
struct MADT_LAPIC
{
	processor: u8,
	apic_id: u8,
	flags: u32,
}
#[repr(C,packed)]
struct MADT_IOAPIC
{
	ioapic_id: u8,
	_resvd: u8,
	address: u32,
	interrupt_base: u32,
}
#[deriving(Show)]
#[repr(C,packed)]
struct MADT_IntSrcOvr
{
	bus: u8,
	source: u8,
	gsi: u32,
	flags: u16,
}
#[deriving(Show)]
#[repr(C,packed)]
struct MADT_NMI
{
	flags: u16,
	gsi: u32,
}
#[deriving(Show)]
#[repr(C,packed)]
struct MADT_LAPICNMI
{
	processor: u8,
	flags: u16,
	lint_num: u8,
}

#[deriving(Show)]
enum MADTDevRecord<'a>
{
	DevUnk(u8),
	DevLAPIC(&'a MADT_LAPIC),
	DevIOAPIC(&'a MADT_IOAPIC),
	DevIntSrcOvr(&'a MADT_IntSrcOvr),
	DevNMI(&'a MADT_NMI),
	DevLAPICNMI(&'a MADT_LAPICNMI),
}

struct MADTRecords<'a>
{
	madt: &'a ACPI_MADT,
	pos: uint,
	limit: uint,
}

#[repr(C)]
struct APICReg
{
	data: u32,
	_rsvd: [u32,..3],
}

struct LAPIC
{
	mapping: ::memory::virt::AllocHandle,
}

struct IOAPIC
{
	mapping: ::memory::virt::AllocHandle,
	num_lines: uint,
	first_irq: uint,
}

#[repr(C)]
enum ApicRegisters
{
	ApicReg_LAPIC_ID  = 0x2,
	ApicReg_LAPIC_Ver = 0x3,
	ApicReg_TPR       = 0x8,
	ApicReg_APR       = 0x9,
	ApicReg_PPR       = 0xA,
}

#[link_section="processor_local"]
static mut s_lapic_lock: ::sync::Mutex<()> = mutex_init!( () );
static mut s_lapic: *const LAPIC = 0 as *const _;
static mut s_ioapics: *const Vec<::sync::Mutex<IOAPIC>> = 0 as *const _;

fn init()
{
	let handles = ::arch::acpi::find::<ACPI_MADT>("APIC");
	if handles.len() == 0 {
		log_warning!("No MADT ('APIC') table in ACPI");
		return ;
	}
	if handles.len() > 1 {
		log_notice!("Multiple MADTs ({})", handles.len());
	}
	
	let madt = &handles[0];
	madt.data().dump(madt.data_len());
	
	if (madt.data().flags & 1) != 0 {
		log_notice!("Legacy PIC present, disabling");
		// TODO: Disable legacy PIC
	}
	
	unsafe {
		s_lapic = ::memory::heap::alloc( LAPIC::new(0xFEC00000) ) as *const _;
		let ioapics: Vec<_> = madt.data().records(madt.data_len()).filter_map(
				|r|match r {
					DevIOAPIC(a) => Some(IOAPIC::new(a.address as u64, a.interrupt_base as uint)),
					_ => None
					}
				).collect();
		s_ioapics = ::memory::heap::alloc( ioapics ) as *const _;
		};
}

impl LAPIC
{
	pub fn new(paddr: u64) -> LAPIC
	{
		let ret = LAPIC {
			mapping: ::memory::virt::map_hw_rw(paddr, 1, "APIC").unwrap(),
			};
		
		log_debug!("ID: {:x}, Ver: {:x}", ret.read_reg(ApicReg_LAPIC_ID), ret.read_reg(ApicReg_LAPIC_Ver));
	
		ret
	}
	
	fn read_reg(&self, idx: ApicRegisters) -> u32
	{
		let regs = self.mapping.as_ref::<[APICReg,..2]>(0);
		regs[0].data = idx as u32;
		regs[1].data
	}
}

impl IOAPIC
{
	pub fn new(paddr: u64, base: uint) -> IOAPIC
	{
		let mut ret = IOAPIC {
			mapping: ::memory::virt::map_hw_rw(paddr, 1, "IOAPIC").unwrap(),
			num_lines: 0,
			first_irq: base,
			};
		
		ret.num_lines = (ret.read_reg(1) & 0xFF) as uint + 1;
		
		ret
	}
	
	fn read_reg(&self, idx: uint) -> u32
	{
		let regs = self.mapping.as_ref::<[APICReg,..2]>(0);
		regs[0].data = idx as u32;
		regs[1].data
	}
}

impl ACPI_MADT
{
	pub fn records(&self, len: uint) -> MADTRecords
	{
		MADTRecords {
			madt: self,
			pos: 0,
			limit: len - ::core::mem::size_of::<ACPI_MADT>()
			}
	}
	fn dump(&self, len: uint)
	{
		log_debug!("MADT = {{");
		log_debug!("  local_controller_addr: {:#x}", self.local_controller_addr);
		log_debug!("  flags: {:#x}", self.flags);
		log_debug!("}}");
		
		for (i,rec) in self.records(len).enumerate()
		{
			log_debug!("@{}: {}", i, rec);
		}
	}
	
	fn get_record<'s>(&'s self, limit: uint, pos: uint) -> (uint,MADTDevRecord)
	{
		assert!(pos < limit);
		assert!(pos + ::core::mem::size_of::<MADT_DevHeader>() <= limit);
		unsafe {
			let ptr = (&self.end as *const u8).offset( pos as int ) as *const MADT_DevHeader;
			log_debug!("pos={}, ptr={} (type={},len={})", pos, ptr, (*ptr).dev_type, (*ptr).rec_len);
			let len = (*ptr).rec_len;
			let typeid = (*ptr).dev_type;
			
			let ret_ref = match typeid {
				0 => DevLAPIC(     ::core::mem::transmute( ptr.offset(1) ) ),
				1 => DevIOAPIC(    ::core::mem::transmute( ptr.offset(1) ) ),
				2 => DevIntSrcOvr( ::core::mem::transmute( ptr.offset(1) ) ),
				3 => DevNMI(       ::core::mem::transmute( ptr.offset(1) ) ),
				4 => DevLAPICNMI(  ::core::mem::transmute( ptr.offset(1) ) ),
				_ => DevUnk(typeid) ,
				};
			
			(pos + len as uint, ret_ref)
		}
	}
}

impl<'a> Iterator<MADTDevRecord<'a>> for MADTRecords<'a>
{
	fn next(&mut self) -> Option<MADTDevRecord<'a>>
	{
		if self.pos >= self.limit
		{
			None
		}
		else
		{
			let (newpos,rec) = self.madt.get_record(self.limit, self.pos);
			self.pos = newpos;
			Some(rec)
		}
	}
}

impl ::core::fmt::Show for MADT_LAPIC
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::FormatError>
	{
		write!(f, "{{Proc:{},APIC:{},Flags:{:#x}}}", self.processor, self.apic_id, self.flags)
	}
}
impl ::core::fmt::Show for MADT_IOAPIC
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::FormatError>
	{
		write!(f, "{{ID:{},Addr:{:#x},BaseIRQ:{}}}", self.ioapic_id, self.address, self.interrupt_base)
	}
}

// vim: ft=rust

