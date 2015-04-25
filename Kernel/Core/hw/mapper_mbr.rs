// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/mapper_mbr.rs
/// Master Boot Record logical volume mapper
use _common::*;

module_define!{MapperMBR, [Storage], init}

static S_MAPPER: Mapper = Mapper;

fn init()
{
	::metadevs::storage::register_mapper(&S_MAPPER);
}

struct Mapper;

impl ::metadevs::storage::Mapper for Mapper
{
	fn name(&self) -> &str { "mbr" }

	fn handles_pv(&self, pv: &::metadevs::storage::PhysicalVolume) -> usize {
		if pv.blocksize() != 512 {
			todo!("Support non 512 byte sectors in MBR mapper (got {})", pv.blocksize());
		}
		
		let mut block: [u8; 512] = unsafe { ::core::mem::zeroed() };
		log_debug!("Reading from PV '{}'", pv.name());
		pv.read(0, 0, 1, &mut block).unwrap().wait();
		
		log_debug!("PV '{}' boot sig {:02x} {:02x}", pv.name(), block[0x1FE], block[0x1FF]);
		if block[0x1FE] == 0x55 && block[0x1FE+1] == 0xAA {
			1
		}
		else {
			0
		}
	}
}

