// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/mapper_mbr.rs
/// Master Boot Record logical volume mapper
use prelude::*;
use lib::byteorder::{ReadBytesExt,LittleEndian};
use metadevs::storage;

module_define!{MapperMBR, [Storage], init}

static S_MAPPER: Mapper = Mapper;

fn init()
{
	storage::register_mapper(&S_MAPPER);
}

struct Mapper;

#[derive(Debug)]
struct Entry
{
	bootable: bool,
	system_id: u8,
	lba_start: u64,
	lba_count: u64,
}

impl storage::Mapper for Mapper
{
	fn name(&self) -> &str { "mbr" }

	fn handles_pv(&self, pv: &storage::PhysicalVolume) -> Result<usize,storage::IoError> {
		if pv.blocksize() != 512 {
			log_log!("Support non 512 byte sectors in MBR mapper (got {})", pv.blocksize());
			return Ok(0);
		}
		
		let mut block: [u8; 512] = unsafe { ::core::mem::zeroed() };
		try!(pv.read(0, 0, 1, &mut block).wait());
		
		log_debug!("PV '{}' boot sig {:02x} {:02x}", pv.name(), block[0x1FE], block[0x1FF]);
		if block[0x1FE] == 0x55 && block[0x1FE+1] == 0xAA {
			Ok(1)
		}
		else {
			Ok(0)
		}
	}
	
	fn enum_volumes(&self, pv: &::metadevs::storage::PhysicalVolume, new_volume_cb: &mut FnMut(String, u64, u64)) -> Result<(),storage::IoError> {
		if !(pv.blocksize() == 512) {
			return Err( storage::IoError::InvalidParameter );
		}
		
		let mut block: [u8; 512] = unsafe { ::core::mem::zeroed() };
		try!( pv.read(0, 0, 1, &mut block).wait() );
		if !(block[510] == 0x55 && block[511] == 0xAA) {
			return Err( storage::IoError::InvalidParameter );
		}
		
		// the "unique ID" (according to the osdev.org wiki) might just be the tail of the MBR code
		//let uid = &block[0x1b4 .. 0x1be];
		
		for i in (0 .. 4) {
			let ofs = 0x1BE + i*16;
			
			if let Some(info) = Entry::read( &block[ofs .. ofs + 16] )
			{
				log_debug!("{:?}", info);
				if info.system_id == 0x5 || info.system_id == 0xF {
					todo!("Extended partition");
				}
				else {
					new_volume_cb( format!("{}p{}", pv.name(), i), info.lba_start, info.lba_count );
				}
			}
		}
		
		Ok( () )
	}
}

impl Entry
{
	fn read(data: &[u8]) -> Option<Entry>
	{
		assert!(data.len() >= 16);
		if data[4] == 0 {
			return None;
		}
		
		if data[0] & 0x7E != 0 {
			log_warning!("Partition entry has reserved bits set in byte 0 {:#x}", data[0]);
			return None;
		}
		
		let (base, len) = if data[0] & 1 != 0 {
				todo!("non-standard 48-bit LBA");
			}
			else {
				let base = (&data[8..]).read_u32::<LittleEndian>().unwrap() as u64;
				let len = (&data[12..]).read_u32::<LittleEndian>().unwrap() as u64;
				(base, len)
			};
		
		Some(Entry {
			bootable: (data[0] & 0x80) != 0,
			system_id: data[4],
			lba_start: base,
			lba_count: len,
			})
	}
}

