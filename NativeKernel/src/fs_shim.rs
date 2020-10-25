use ::kernel::vfs::mount;
use ::kernel::vfs::node::InodeId;
use ::kernel::vfs;
use ::kernel::metadevs::storage::VolumeHandle;
use ::std::path::{Path,PathBuf};
use ::kernel::lib::byte_str::{ByteStr/*,ByteString*/};
use ::kernel::lib::mem::aref::{Aref,ArefBorrow};

pub struct NativeFsDriver;

#[derive(Default)]
struct NativeFs
{
    inner: ::std::sync::Mutex< NativeFsInner >,
}
#[derive(Default)]
struct NativeFsInner
{
    inodes: ::std::collections::HashMap<InodeId, Box<EntData>>,
    last_inode: InodeId,
}
enum EntData
{
    Dir(DirData),
    File(FileData),
}
struct DirData
{
    path: PathBuf,
}
struct FileData
{
    path: PathBuf,
}

impl mount::Driver for NativeFsDriver
{
	fn detect(&self, vol: &VolumeHandle) -> vfs::Result<usize> {
        Ok(0)
    }
	fn mount(&self, vol: VolumeHandle, handle: mount::SelfHandle) -> vfs::Result<Box<dyn mount::Filesystem>> {
        let root_path: PathBuf = "../Usermode/.output/native".into();
        let mut rv = NativeFs::default();
        rv.inner.get_mut().unwrap().inodes.insert(0, Box::new(EntData::Dir(DirData{ path: root_path })));
        Ok(Box::new(FsInstance(Aref::new(rv))))
    }
}

impl NativeFs
{
    fn get_dir(&self, inode_id: InodeId) -> &DirData {
        match **self.inner.lock().unwrap().inodes.get(&inode_id).expect("")
        {
        EntData::Dir(ref r) => unsafe { &*(r as *const _) },
        _ => panic!(""),
        }
    }
    fn allocate_inode(&self, data: EntData) -> InodeId {
        let mut lh = self.inner.lock().unwrap();
        lh.last_inode += 1;
        let rv = lh.last_inode;
        lh.inodes.insert(rv, Box::new(data));
        rv
    }
}
struct FsInstance(Aref<NativeFs>);
impl mount::Filesystem for FsInstance
{
	fn root_inode(&self) -> InodeId {
        0
    }
	fn get_node_by_inode(&self, n: InodeId) -> Option<vfs::node::Node> {
        Some(match &**self.0.inner.lock().unwrap().inodes.get(&n)?
        {
        EntData::Dir(r) => vfs::node::Node::Dir(Box::new(DirNodeRef( self.0.borrow(), n ))),
        EntData::File(r) => vfs::node::Node::File(Box::new(FileNodeRef( self.0.borrow(), n ))),
        })
    }
}
#[derive(Clone)]
struct DirNodeRef(ArefBorrow<NativeFs>, InodeId);
impl vfs::node::NodeBase for DirNodeRef
{
	/// Return the volume's inode number
	fn get_id(&self) -> InodeId {
        self.1
    }
	/// Return an &Any associated with this node (not nessesarily same as `self`, up to the driver)
	fn get_any(&self) -> &dyn ::std::any::Any {
        self
    }
}
impl vfs::node::Dir for DirNodeRef
{
	/// Acquire a node given the name
	fn lookup(&self, name: &ByteStr) -> vfs::node::Result<InodeId> {
        let dir_info = self.0.get_dir(self.1);
        let name = std::str::from_utf8(name.as_bytes()).unwrap();
        let path: PathBuf = [&dir_info.path, Path::new(name)].iter().collect();
        log_debug!("path = {:?}", path);
        Ok(self.0.allocate_inode(if path.is_dir() {
                EntData::Dir(DirData {
                    path,
                    })
            }
            else if path.is_file() {
                EntData::File(FileData {
                    path,
                    })
            }
            else {
                todo!("lookup({:?}): {:?}", name, path);
            }))
    }
	
	/// Read Entry
	/// 
	/// Returns:
	/// - Ok(Next Offset)
	/// - Err(e) : Any error
	fn read(&self, start_ofs: usize, callback: &mut vfs::node::ReadDirCallback) -> vfs::node::Result<usize> {
        todo!("read")
    }
	
	/// Create a new file in this directory
	/// 
	/// Returns the newly created node
	fn create(&self, name: &ByteStr, nodetype: vfs::node::NodeType) -> vfs::node::Result<InodeId> {
        todo!("create")
    }
	/// Create a new name for the provided inode
	fn link(&self, name: &ByteStr, inode: &dyn vfs::node::NodeBase) -> vfs::node::Result<()> {
        todo!("link")
    }
	/// Remove the specified name
	fn unlink(&self, name: &ByteStr) -> vfs::node::Result<()> {
        todo!("unlink")
    }
}

#[derive(Clone)]
struct FileNodeRef(ArefBorrow<NativeFs>, InodeId);
impl vfs::node::NodeBase for FileNodeRef
{
	/// Return the volume's inode number
	fn get_id(&self) -> InodeId {
        self.1
    }
	/// Return an &Any associated with this node (not nessesarily same as `self`, up to the driver)
	fn get_any(&self) -> &dyn ::std::any::Any {
        self
    }
}
impl vfs::node::File for FileNodeRef
{
	/// Returns the size (in bytes) of this file
	fn size(&self) -> u64 {
        todo!("size")
    }
	/// Update the size of the file (zero padding or truncating)
	fn truncate(&self, newsize: u64) -> vfs::node::Result<u64> {
        todo!("truncate")
    }
	/// Clear the specified range of the file (replace with zeroes)
	fn clear(&self, ofs: u64, size: u64) -> vfs::node::Result<()> {
        todo!("clear")
    }
	/// Read data from the file
	fn read(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize> {
        todo!("read")
    }
	/// Write data to the file, can only grow the file if ofs==size
    fn write(&self, ofs: u64, buf: &[u8]) -> vfs::node::Result<usize> {
        todo!("write")
    }
}