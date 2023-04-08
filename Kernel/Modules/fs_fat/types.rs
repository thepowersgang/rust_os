
#[derive(Copy,Clone,PartialEq,Eq)]
#[derive(PartialOrd,Ord)]
pub struct ClusterNum(::core::num::NonZeroU32);
impl ClusterNum {
	pub fn new(v: u32) -> Result<ClusterNum,()> {
		// Clusters 0 and 1 aren't valid
		// FAT32 uses up to 24 bits for the cluster numbers
		if 2 <= v && v <= 0xFF_FFFF {
			Ok( ClusterNum(::core::num::NonZeroU32::new(v).unwrap()) )
		}
		else {
			log_error!("Invalid cluster number! {:#x} out of 2 .. 0xFF_FFFF", v);
			Err( () )
		}
	}
	pub fn get(&self) -> u32 {
		self.0.get()
	}
}
impl ::core::fmt::Display for ClusterNum {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "C{:#x}", self.get())
	}
}
impl ::core::fmt::Debug for ClusterNum {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		::core::fmt::Display::fmt(self, f)
	}
}


// --------------------------------------------------------------------
/// Inodes IDs destrucure into two 28-bit cluster IDs, and a 16-bit dir offset
#[derive(Debug)]
pub(crate) struct InodeRef
{
	pub dir_first_cluster: Option<ClusterNum>,
	pub first_cluster: ClusterNum,
}

impl InodeRef
{
	pub(crate) fn root(root_dir_c: ClusterNum) -> InodeRef {
		InodeRef {
			first_cluster: root_dir_c,
			dir_first_cluster: None,
		}
	}
	pub(crate) fn new(file_c: ClusterNum, dir_c: ClusterNum) -> InodeRef {
		InodeRef {
			first_cluster: file_c,
			dir_first_cluster: Some(dir_c),
		}
	}
	pub(crate) fn to_id(&self) -> ::vfs::node::InodeId {
		(self.first_cluster.get() as u64)
		| (self.dir_first_cluster.map(|v| v.get()).unwrap_or(0) as u64) << 24
	}
}

impl From<::vfs::node::InodeId> for InodeRef {
	fn from(v: ::vfs::node::InodeId) -> InodeRef {
		InodeRef {
			first_cluster: ClusterNum::new( (v & 0x00FF_FFFF) as u32 ).expect("Invalid InodeID (cluster)"),
			dir_first_cluster: {
				let v = ((v >> 24) & 0x00FF_FFFF) as u32;
				if v == 0 { None } else { Some(ClusterNum::new(v).expect("Invalid InodeId (dir)")) }
				},
		}
	}
}

// --------------------------------------------------------------------
/// Iterable cluster list
pub(crate) enum ClusterList<'fs> {
	Range(::core::ops::Range<u32>),
	Chained(&'fs super::FilesystemInner, Option<ClusterNum>),
}


impl<'fs> ClusterList<'fs> {
	pub fn range(start: ClusterNum, count: u32) -> ClusterList<'fs> {
		ClusterList::Range(start.get() .. start.get() + count)
	}
	pub fn chained(fs: &'fs super::FilesystemInner, start: ClusterNum) -> ClusterList<'fs> {
		ClusterList::Chained(fs, Some(start))
	}

	/// Returns an extent of at most `max_clusters` contigious clusters
	pub fn next_extent(&mut self, max_clusters: usize) -> Option<(ClusterNum, usize)> {
		match *self
		{
		ClusterList::Range(ref mut r) => {
			let rv = r.start;
			let count = u32::min( r.start - r.end, max_clusters as u32 );
			if count == 0 {
				None
			}
			else {
				r.start += count;
				Some( (ClusterNum::new(rv).unwrap(), count as usize) )
			}
			},
		ClusterList::Chained(ref fs, ref mut next) =>
			match *next
			{
			None => None,
			Some(rv) => {
				let mut count = 0;
				let mut next_cluster = rv;
				while *next == Some(next_cluster) && count < max_clusters
				{
					*next = match fs.get_next_cluster(next_cluster)
						{
						Ok(opt_v) => opt_v,
						Err(e) => {
							log_warning!("Error when reading cluster chain - {:?}", e);
							return None;	// Inconsistency, terminate asap
							},
						};
					next_cluster = ClusterNum::new(next_cluster.get() + 1).unwrap();
					count += 1;
				}
				assert!(count > 0);
				Some( (rv, count) )
				}
			},
		}
	}
}

impl<'fs> ::core::iter::Iterator for ClusterList<'fs> {
	type Item = ClusterNum;
	fn next(&mut self) -> Option<Self::Item> {
		match *self
		{
		ClusterList::Range(ref mut r) => r.next().map(|c| ClusterNum::new(c).unwrap()),
		ClusterList::Chained(ref fs, ref mut next) =>
			match next.take()
			{
			Some(rv) => {
				*next = match fs.get_next_cluster(rv)
					{
					Ok(opt_v) => opt_v,
					Err(e) => {
						log_warning!("Error when reading cluster chain - {:?}", e);
						return None;	// Inconsistency, terminate asap
						},
					};
				Some( rv )
				},
			None => None,
			},
		}
	}
}
