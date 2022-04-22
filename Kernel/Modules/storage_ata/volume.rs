// "Tifflin" Kernel - ATA Driver
// - By John Hodge (thePowersGang)
//
// Modules/storage_ata/volume.rs
//! Generic ATA volume support
use ::kernel::prelude::*;
use ::kernel::metadevs::storage::{self, DataPtr};

pub struct Error(u8);
impl From<Error> for storage::IoError
{
	fn from(_v: Error) -> storage::IoError
	{
		storage::IoError::Unknown("ATA")
	}
}
impl_from! {
	From<u8>(v) for Error {
		Error(v)
	}
}

//const ATA_IDENTIFY_DEVICE: u8 = 0xEC;

const ATA_READ_DMA: u8 = 0xC8;
const ATA_WRITE_DMA: u8 = 0xCA;
const ATA_READ_DMA_EXT: u8 = 0x25;
const ATA_WRITE_DMA_EXT: u8 = 0x35;

pub trait Interface: 'static + Send
{
	fn name(&self) -> &str;

	fn ata_identify(&self) -> Result<super::AtaIdentifyData, Error>;
	fn dma_lba_28(&self, cmd: u8, count: u8 , addr: u32, data: DataPtr) -> Result<usize,Error>;
	fn dma_lba_48(&self, cmd: u8, count: u16, addr: u64, data: DataPtr) -> Result<usize,Error>;
}

pub struct AtaVolume<I: Interface>
{
	int: I,
	block_size: u32,
	block_count: u64,
}

impl<I: Interface> AtaVolume<I>
{
	pub fn new_boxed(int: I) -> Result<Box<Self>, storage::IoError>
	{
		let ident_data = int.ata_identify()?;

		let block_size = if ident_data.words_per_logical_sector == 0 { 512 } else { ident_data.words_per_logical_sector as u32 * 2 };
		let block_count = if ident_data.sector_count_28 == 0 { ident_data.sector_count_48 } else { ident_data.sector_count_28 as u64 };
		
		log_log!("{}: Hard Disk, {} sectors of {}b each, {}", int.name(), block_count, block_size, storage::SizePrinter(block_count * block_size as u64));
				
		Ok(Box::new(AtaVolume {
			int: int,
			block_size: block_size,
			block_count: block_count,
			}))
	}
}


impl<I: Interface + Send + 'static> storage::PhysicalVolume for AtaVolume<I>
{
	fn name(&self) -> &str { self.int.name() }
	fn blocksize(&self) -> usize { self.block_size as usize }
	fn capacity(&self) -> Option<u64> { Some(self.block_count) }
	
	fn read<'a>(&'a self, _prio: u8, idx: u64, num: usize, dst: &'a mut [u8]) -> storage::AsyncIoResult<'a,usize>
	{
		assert_eq!( dst.len(), num * self.block_size as usize );
		let ret = if idx < (1 << 28) && num < 256 {
				self.int.dma_lba_28(ATA_READ_DMA, num as u8, idx as u32, DataPtr::Recv(dst))
			}
			else if idx < (1 << 48) && num < (1 << 16) {
				self.int.dma_lba_48(ATA_READ_DMA_EXT, num as u16, idx, DataPtr::Recv(dst))
			}
			else {
				panic!("Count/address out of range for ATA");
			};
		let ret = ret.map_err(|e| e.into());

		Box::pin(async move { ret })
	}
	fn write<'a>(&'a self, _prio: u8, idx: u64, num: usize, src: &'a [u8]) -> storage::AsyncIoResult<'a,usize>
	{
		assert_eq!( src.len(), num * self.block_size as usize );
		let ret = if idx < (1 << 28) && num < 256 {
				self.int.dma_lba_28(ATA_WRITE_DMA, num as u8, idx as u32, DataPtr::Send(src))
			}
			else if idx < (1 << 48) && num < (1 << 16) {
				self.int.dma_lba_48(ATA_WRITE_DMA_EXT, num as u16, idx, DataPtr::Send(src))
			}
			else {
				panic!("Count/address out of range for ATA");
			};
		let ret = ret.map_err(|e| e.into());

		Box::pin(async move { ret })
	}
	
	fn wipe<'a>(&'a self, _blockidx: u64, _count: usize) -> storage::AsyncIoResult<'a,()>
	{
		// Do nothing, no support for TRIM
		Box::pin(async move { Ok(()) })
	}
	
}
