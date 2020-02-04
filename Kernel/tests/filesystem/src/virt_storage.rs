//! 
use ::kernel::metadevs::storage;
use ::kernel::{log,log_log,log_error};

pub fn add_volume(name: &str, path: &::std::path::Path) -> Result<(), ::std::io::Error>
{
    use ::std::io::{Seek};
    let block_size = 512;

    let name = name.to_owned();

    let mut fp = ::std::fs::File::open(path)?;
    let byte_count = fp.seek(::std::io::SeekFrom::End(0))?;
    let block_count = byte_count / block_size as u64;

    log_log!("{}: {} ({:#x} bytes, {:#x} blocks)", name, path.display(), byte_count, block_count);

    let h = storage::register_pv( Box::new(Volume {
        name: name.clone(),
        block_size: block_size,
        block_count: block_count,
        fp: ::std::sync::Mutex::new(fp),
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
}

impl Volume
{
    fn read_inner(&self, idx: u64, dst: &mut [u8]) -> ::std::io::Result<usize>
    {
        use ::std::io::{Seek,Read};
        let mut lh = self.fp.lock().unwrap();
        lh.seek(::std::io::SeekFrom::Start( idx * self.block_size as u64 ))?;
        Ok(lh.read(dst)? / self.block_size)
    }
    fn write_inner(&self, idx: u64, src: &[u8]) -> ::std::io::Result<usize>
    {
        use ::std::io::{Seek,Write};
        let mut lh = self.fp.lock().unwrap();
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

		Box::new( ::kernel::r#async::NullResultWaiter::new( move || ret ) )
	}
	fn write<'a>(&'a self, _prio: u8, idx: u64, num: usize, src: &'a [u8]) -> storage::AsyncIoResult<'a,usize>
	{
        assert_eq!( src.len(), num * self.block_size );
        let ret = self.write_inner(idx, src).map_err(cvt_err);

		Box::new( ::kernel::r#async::NullResultWaiter::new( move || ret ) )
	}
	
	fn wipe<'a>(&'a self, _blockidx: u64, _count: usize) -> storage::AsyncIoResult<'a,()>
	{
		// Do nothing, no support for TRIM
		Box::new( ::kernel::r#async::NullResultWaiter::new( || Ok( () ) ))
	}
	
}