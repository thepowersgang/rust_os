//! 
use ::kernel::metadevs::storage;
use ::kernel::{log,log_log,log_error};

pub enum OverlayType
{
    None,
    Temporary,
    Persistent,
}

pub fn add_volume(name: &str, path: &::std::path::Path, overlay_ty: OverlayType) -> Result<()/*::kernel::metadevs::storage::PhysicalVolumeReg*/, ::std::io::Error>
{
    use ::std::io::{Seek};
    let block_size = 512;

    let name = name.to_owned();

    let mut fp = ::std::fs::File::open(path)?;
    let byte_count = fp.seek(::std::io::SeekFrom::End(0))?;
    let block_count = byte_count / block_size as u64;

    log_log!("{}: {} ({:#x} bytes, {:#x} blocks)", name, path.display(), byte_count, block_count);

    let overlay = match overlay_ty
        {
        OverlayType::None => None,
        OverlayType::Temporary => Some(Overlay::create(block_count as usize, block_size, &path.with_extension("tmp-overlay"))?),
        OverlayType::Persistent => Some(Overlay::load(block_count as usize, block_size, &path.with_extension("overlay"))?),
        };

    let h = storage::register_pv( Box::new(Volume {
        name: name.clone(),
        block_size: block_size,
        block_count: block_count,
        fp: ::std::sync::Mutex::new(fp),
        write_overlay: overlay,
        }) );
    ::std::mem::forget(h);
    Ok( () )
}

struct Volume
{
    name: String,
    block_size: usize,
    block_count: u64,
    fp: ::std::sync::Mutex< ::std::fs::File>,
    write_overlay: Option<Overlay>,
}
struct Overlay
{
    blocks: ::std::sync::Mutex< ::bitvec::vec::BitVec<bitvec::order::Lsb0, u8> >,
    fp: ::std::sync::Mutex< ::std::fs::File>,
}
impl Overlay
{
    fn load(count: usize, blocksize: usize, path: &::std::path::Path) -> ::std::io::Result<Overlay>
    {
        use ::std::io::{Seek,SeekFrom,Read};
        let mut bmp = ::bitvec::vec::BitVec::new();
        bmp.resize(count, false);
        let mut fp = ::std::fs::OpenOptions::new()
            .create(true).read(true).write(true)
            .open(path)?
            ;
        let exp_len = count as u64 * blocksize as u64 + (count+8-1) as u64 / 8;
        let len = fp.seek(SeekFrom::End(0))?;
        if len == 0 {
            fp.set_len(exp_len)?;
        }
        else if len == exp_len {
            // Read the saved bitmap
            fp.seek(SeekFrom::Start(count as u64 * blocksize as u64))?;
            fp.read(bmp.as_mut_slice())?;
        }
        else {
            panic!("TODO: Error when overlay size doesn't match image size")
        }
        Ok(Overlay {
            blocks: ::std::sync::Mutex::new(bmp),
            fp: ::std::sync::Mutex::new(fp),
            })
    }
    fn create(count: usize, blocksize: usize, path: &::std::path::Path) -> ::std::io::Result<Overlay>
    {
        let mut bmp = ::bitvec::vec::BitVec::new();
        bmp.resize(count, false);
        let fp = ::std::fs::File::create(path)?;
        fp.set_len(count as u64 * blocksize as u64 + (count+8-1) as u64 / 8)?;
        Ok(Overlay {
            blocks: ::std::sync::Mutex::new(bmp),
            fp: ::std::sync::Mutex::new(fp),
            })
    }
}

impl Volume
{
    fn read_inner(&self, idx: u64, dst: &mut [u8]) -> ::std::io::Result<usize>
    {
        use ::std::io::{Seek,Read};
        // If there's a write overlay, read from that if the specified block is in it.
        if let Some(ref overlay) = self.write_overlay
        {
            let mut base_lh = self.fp.lock().unwrap();
            let mut over_lh = overlay.fp.lock().unwrap();
            let over_bm = overlay.blocks.lock().unwrap();
            for (b,dst) in Iterator::enumerate(dst.chunks_mut(self.block_size))
            {
                if let Some(true) = over_bm.get(idx as usize + b)
                {
                    over_lh.seek(::std::io::SeekFrom::Start( (idx + b as u64) * self.block_size as u64 ))?;
                    over_lh.read(dst)?;
                }
                else
                {
                    base_lh.seek(::std::io::SeekFrom::Start( (idx + b as u64) * self.block_size as u64 ))?;
                    base_lh.read(dst)?;
                }
            }
            Ok( dst.len() / self.block_size )
        }
        else
        {
            let mut lh = self.fp.lock().unwrap();
            lh.seek(::std::io::SeekFrom::Start( idx * self.block_size as u64 ))?;
            Ok(lh.read(dst)? / self.block_size)
        }
    }
    fn write_inner(&self, idx: u64, src: &[u8]) -> ::std::io::Result<usize>
    {
        // Optional write overlay?
        use ::std::io::{Seek,Write};
        let mut lh = if let Some(ref overlay) = self.write_overlay
            {
                let nblks = src.len() / self.block_size;
                let mut over_bm = overlay.blocks.lock().unwrap();
                if (over_bm.len() as u64) < idx + nblks as u64 {
                    assert!( (idx + nblks as u64) < usize::max_value() as u64 );
                    over_bm.resize( idx as usize + nblks, false );
                }
                over_bm[idx as usize..][..nblks].set_all(true);
                overlay.fp.lock().unwrap()
            }
            else
            {
                self.fp.lock().unwrap()
            };
        lh.seek(::std::io::SeekFrom::Start( idx * self.block_size as u64 ))?;
        Ok(lh.write(src)? / self.block_size)
    }
}

fn cvt_err(e: ::std::io::Error) -> storage::IoError
{
    match e.kind()
    {
    _ => {
        log_error!("cvt_error: unknown error {:?}", e);
        storage::IoError::Unknown("?")
        },
    }
}

impl storage::PhysicalVolume for Volume
{
	fn name(&self) -> &str { &self.name }
	fn blocksize(&self) -> usize { self.block_size }
	fn capacity(&self) -> Option<u64> { Some(self.block_count) }
	
	fn read<'a>(&'a self, _prio: u8, idx: u64, num: usize, dst: &'a mut [u8]) -> storage::AsyncIoResult<'a,usize>
	{
        assert_eq!( dst.len(), num * self.block_size );
        let ret = self.read_inner(idx, dst).map_err(cvt_err);

		Box::pin( ::core::future::ready(ret) )
	}
	fn write<'a>(&'a self, _prio: u8, idx: u64, num: usize, src: &'a [u8]) -> storage::AsyncIoResult<'a,usize>
	{
        assert_eq!( src.len(), num * self.block_size );
        let ret = self.write_inner(idx, src).map_err(cvt_err);

		Box::pin( ::core::future::ready(ret) )
	}
	
	fn wipe<'a>(&'a self, _blockidx: u64, _count: usize) -> storage::AsyncIoResult<'a,()>
	{
		// Do nothing, no support for TRIM
        let ret = Ok(());
		Box::pin( ::core::future::ready(ret) )
	}
	
}