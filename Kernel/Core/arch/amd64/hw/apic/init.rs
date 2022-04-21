// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/apic/init.rs
// - x86 APIC Initialisation (ACPI parsing)
#[allow(unused_imports)]
use crate::prelude::*;

#[repr(C,packed)]
//#[derive(FromPacked)]
pub struct ACPI_MADT
{
	pub local_controller_addr: u32,
	pub flags: u32,
	end: [u8; 0],
}
#[repr(C,packed)]
struct MADT_DevHeader
{
	dev_type: u8,
	rec_len: u8,
}
#[repr(C,packed)]
pub struct MADT_LAPIC
{
	processor: u8,
	apic_id: u8,
	pub flags: u32,
}
#[repr(C,packed)]
pub struct MADT_IOAPIC
{
	ioapic_id: u8,
	_resvd: u8,
	pub address: u32,
	pub interrupt_base: u32,
}
#[derive(Copy,Clone)]
#[derive(Debug)]
#[repr(C,packed)]
pub struct MADT_IntSrcOvr
{
	bus: u8,
	source: u8,
	gsi: u32,
	flags: u16,	// Polarity (2), Trigger Mode (2), MBZ(12)
}
#[derive(Copy,Clone)]
#[derive(Debug)]
#[repr(C,packed)]
pub struct MADT_NMI
{
	flags: u16,
	gsi: u32,
}
#[derive(Copy,Clone)]
#[derive(Debug)]
#[repr(C,packed)]
pub struct MADT_LAPICNMI
{
	processor: u8,
	flags: u16,
	lint_num: u8,
}
#[derive(Copy,Clone)]
#[derive(Debug)]
#[repr(C,packed)]
pub struct MADT_LAPICAddr
{
	_rsvd: u16,
	pub address: u64,
}

#[derive(Debug)]
pub enum MADTDevRecord<'a>
{
	DevUnk(u8),
	DevLAPIC(&'a MADT_LAPIC),
	DevIOAPIC(&'a MADT_IOAPIC),
	DevIntSrcOvr(&'a MADT_IntSrcOvr),
	DevNMI(&'a MADT_NMI),
	DevLAPICNMI(&'a MADT_LAPICNMI),
	DevLAPICAddr(&'a MADT_LAPICAddr),
}

pub struct MADTRecords<'a>
{
	madt: &'a ACPI_MADT,
	pos: usize,
	limit: usize,
}

impl ACPI_MADT
{
	pub fn records(&self, len: usize) -> MADTRecords
	{
		MADTRecords {
			madt: self,
			pos: 0,
			limit: len - ::core::mem::size_of::<ACPI_MADT>()
			}
	}
	pub fn dump(&self, len: usize)
	{
		log_debug!("MADT = {{");
		log_debug!("  local_controller_addr: {:#x}", { self.local_controller_addr });
		log_debug!("  flags: {:#x}", { self.flags });
		log_debug!("}}");
		
		for (i,rec) in self.records(len).enumerate()
		{
			log_debug!("@{}: {:?}", i, rec);
		}
	}
	
	unsafe fn get_record<'s>(&'s self, limit: usize, pos: usize) -> (usize, MADTDevRecord)
	{
		assert!(pos < limit);
		assert!(pos + ::core::mem::size_of::<MADT_DevHeader>() <= limit);
		let ptr = (&self.end as *const u8).offset( pos as isize ) as *const MADT_DevHeader;
		//log_debug!("pos={}, ptr={} (type={},len={})", pos, ptr, (*ptr).dev_type, (*ptr).rec_len);
		let len = (*ptr).rec_len;
		let typeid = (*ptr).dev_type;
		
		let ret_ref = match typeid {
			0 => MADTDevRecord::DevLAPIC(     &*(ptr.offset(1) as *const _) ),
			1 => MADTDevRecord::DevIOAPIC(    &*(ptr.offset(1) as *const _) ),
			2 => MADTDevRecord::DevIntSrcOvr( &*(ptr.offset(1) as *const _) ),
			3 => MADTDevRecord::DevNMI(       &*(ptr.offset(1) as *const _) ),
			4 => MADTDevRecord::DevLAPICNMI(  &*(ptr.offset(1) as *const _) ),
			5 => MADTDevRecord::DevLAPICAddr( &*(ptr.offset(1) as *const _) ),
			_ => MADTDevRecord::DevUnk(typeid) ,
			};
		
		(pos + len as usize, ret_ref)
	}
}

impl<'a> Iterator for MADTRecords<'a>
{
	type Item = MADTDevRecord<'a>;
	fn next(&mut self) -> Option<MADTDevRecord<'a>>
	{
		if self.pos >= self.limit
		{
			None
		}
		else
		{
			// SAFE: Assuming we got sane values, this is correct
			let (newpos,rec) = unsafe { self.madt.get_record(self.limit, self.pos) };
			self.pos = newpos;
			Some(rec)
		}
	}
}

impl ::core::fmt::Debug for MADT_LAPIC
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "{{Proc:{},APIC:{},Flags:{:#x}}}", self.processor, self.apic_id, { self.flags })
	}
}
impl ::core::fmt::Debug for MADT_IOAPIC
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "{{ID:{},Addr:{:#x},BaseIRQ:{}}}", self.ioapic_id, { self.address }, { self.interrupt_base })
	}
}

// vim: ft=rust
