//!
//!
//!

pub struct File {
	instance: super::instance::InstanceRef,
	mft_ent: super::instance::CachedMft,

	attr_data: Option<super::ondisk::AttrHandle>,
}

impl File
{
	pub fn new(instance: super::instance::InstanceRef, mft_ent: super::instance::CachedMft) -> Self {
		File {
			attr_data: instance.get_attr_inner(&mft_ent, crate::ondisk::FileAttr::Data, "", 0),
			instance,
			mft_ent,
		}
	}
}

impl ::vfs::node::NodeBase for File
{
	fn get_id(&self) -> u64 {
		todo!("File::get_id")
	}
	fn get_any(&self) -> &(dyn ::core::any::Any + 'static) {
		self
	}
}
impl ::vfs::node::File for File
{
	fn size(&self) -> u64 {
		todo!("File::size")
	}
	fn truncate(&self, _new_size: u64) -> Result<u64, ::vfs::Error> {
		Err(::vfs::Error::ReadOnlyFilesystem)
	}
	fn clear(&self, _ofs: u64, _size: u64) -> Result<(), ::vfs::Error> {
		Err(::vfs::Error::ReadOnlyFilesystem)
	}
	fn read(&self, ofs: u64, dst: &mut [u8]) -> Result<usize, ::vfs::Error> {
		todo!("File::read")
	}
	fn write(&self, _ofs: u64, _src: &[u8]) -> Result<usize, ::vfs::Error> {
		Err(::vfs::Error::ReadOnlyFilesystem)
	}
}
