// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/amd64/tss.rs
//! Initialisation and management of the x86 task state segment
use super::MAX_CPUS;

#[repr(C,packed)]
struct TSS
{
	_resvd1: u32,
	rsp0: u64,
	rsp1: u64,
	rsp2: u64,
	_resvd2: [u32; 2],
	ists: [u64; 7],
	_resvd3: [u32; 2],
	_resvd4: u16,
	io_map_base_ofs: u16,
}
#[repr(C,packed)]
struct GDTEnt(u32,u32);

extern "C" {
	static mut GDT: [GDTEnt; 7+MAX_CPUS*2];
	static mut TSSes: [TSS; MAX_CPUS];
	static EmergencyStack: [u8; 0];
	
	static s_tid0_tls_base: u64;
}

pub fn init()
{
	// SAFE: Module initialisation is single-threaded.
	unsafe {
		for i in 0 .. MAX_CPUS
		{
			TSSes[i].ists[0] = &EmergencyStack as *const _ as u64;
			GDT[7+i*2+0] = GDTEnt::tss_lower( &TSSes[i] );
			GDT[7+i*2+1] = GDTEnt::tss_upper( &TSSes[i] );
		}
		TSSes[0].rsp0 = s_tid0_tls_base as u64;
	}
	
	// SAFE: Just setting the task register
	unsafe {
		::core::arch::asm!("ltr {:x}", in(reg) 7*8_u16);
	}
}


impl GDTEnt
{
	fn tss_lower(ptr: *const TSS) -> GDTEnt {
		// TODO: Support the IOPB?
		let limit = ::core::mem::size_of::<TSS>() - 1;
		let base = ptr as usize;
		
		let low_dword = limit | (base & 0xFFFF) << 16;
		let high_dword = ((base >> 16) & 0xFFFF) | 0x00008900 | (base & 0xFF000000);
		GDTEnt( low_dword as u32, high_dword as u32 )
	}
	fn tss_upper(ptr: *const TSS) -> GDTEnt {
		let base = ptr as usize;
		GDTEnt( (base >> 32) as u32, 0 )
	}
}
