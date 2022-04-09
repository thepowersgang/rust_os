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
	fn detect(&self, _vol: &VolumeHandle) -> vfs::Result<usize> {
        Ok(0)
    }
	fn mount(&self, _vol: VolumeHandle, _handle: mount::SelfHandle) -> vfs::Result<Box<dyn mount::Filesystem>> {
        // TODO: Can this get the path from the volume handle?
        let root_path: PathBuf = ".native_fs".into();
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
    fn get_file(&self, inode_id: InodeId) -> &FileData {
        match **self.inner.lock().unwrap().inodes.get(&inode_id).expect("")
        {
        EntData::File(ref r) => unsafe { &*(r as *const _) },
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
        EntData::Dir(_r) => vfs::node::Node::Dir(Box::new(DirNodeRef( self.0.borrow(), n ))),
        EntData::File(_r) => vfs::node::Node::File(Box::new(FileNodeRef( self.0.borrow(), n ))),
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
        let mut path: PathBuf = [&dir_info.path, Path::new(name)].iter().collect();
        log_debug!("path = {:?}", path);
        #[cfg(windows)]
        if !path.exists()
        {
            let mut p2 = path.clone();
            p2.set_extension("exe");
            if p2.is_file() {
                path = p2;
            }
        }
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
                return Err(vfs::Error::NotFound)
            }))
    }
	
	/// Read Entry
	/// 
	/// Returns:
	/// - Ok(Next Offset)
	/// - Err(e) : Any error
	fn read(&self, start_ofs: usize, callback: &mut vfs::node::ReadDirCallback) -> vfs::node::Result<usize> {
        let dir_info = self.0.get_dir(self.1);
        let read_dir = match ::std::fs::read_dir(&dir_info.path)
            {
            Err(e) => todo!("DirNodeRef::read - read_dir failed {:?}", e),
            Ok(v) => v,
            };
        match read_dir.skip(start_ofs).next()
        {
        None => Err(vfs::Error::NotFound),
        Some(Err(e)) => todo!("DirNodeRef::read - read_dir.next failed {:?}", e),
        Some(Ok(ent)) => {
            let path = ent.path();
            let node_id = self.0.allocate_inode(if path.is_dir() {
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
                    log_error!("Non dir/file encountered?");
                    return Err(vfs::Error::NotFound)
                });
            (callback)(node_id, &mut ent.file_name().to_str().ok_or(vfs::Error::InconsistentFilesystem)?.as_bytes().iter().copied());
            Ok(start_ofs + 1)
            }
        }
    }
	
	/// Create a new file in this directory
	/// 
	/// Returns the newly created node
	fn create(&self, name: &ByteStr, nodetype: vfs::node::NodeType) -> vfs::node::Result<InodeId> {
        todo!("create({:?}, {:?})", name, nodetype)
    }
	/// Create a new name for the provided inode
	fn link(&self, name: &ByteStr, inode: &dyn vfs::node::NodeBase) -> vfs::node::Result<()> {
        todo!("link({:?}, {:?})", name, inode.get_id())
    }
	/// Remove the specified name
	fn unlink(&self, name: &ByteStr) -> vfs::node::Result<()> {
        todo!("unlink({:?})", name)
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
	fn truncate(&self, _newsize: u64) -> vfs::node::Result<u64> {
        todo!("truncate")
    }
	/// Clear the specified range of the file (replace with zeroes)
	fn clear(&self, _ofs: u64, _size: u64) -> vfs::node::Result<()> {
        todo!("clear")
    }
	/// Read data from the file
	fn read(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize> {
        log_debug!("FileNodeRef::read(ofs={:#x}, buf={})", ofs, buf.len());
        use std::io::{Read,Seek,SeekFrom};
        let file_info = self.0.get_file(self.1);
        let mut fp = ::std::fs::File::open(&file_info.path).unwrap();
        fp.seek(SeekFrom::Start(ofs)).map_err(map_err)?;
        Ok( fp.read(buf).map_err(map_err)? )
    }
	/// Write data to the file, can only grow the file if ofs==size
    fn write(&self, _ofs: u64, _buf: &[u8]) -> vfs::node::Result<usize> {
        todo!("write")
    }
}

fn map_err(e: ::std::io::Error) -> vfs::Error {
    todo!("Transform error {:?}", e)
}