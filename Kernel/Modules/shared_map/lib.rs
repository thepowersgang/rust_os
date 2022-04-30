// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/shared_map/lib.rs
//! A key-value map that internally handles synchronisation
//!
//! A wrapper around RwLock<VecMap>
#![no_std]
use ::kernel::sync::rwlock::{RwLock, self};
use ::kernel::lib::collections::VecMap;

extern crate kernel;

pub struct SharedMap<K,V>
{
	lock: RwLock<SharedMapInner<K,V,>>,
}
struct SharedMapInner<K, V>
{
	m: VecMap<K,V>,
}

impl<K, V> SharedMap<K,V>
{
	pub const fn new() -> Self {
		SharedMap {
			lock: RwLock::new(SharedMapInner { m: VecMap::new_const() }),
			}
	}
}
impl<K: Send+Sync+Ord, V: Send+Sync> SharedMap<K,V>
{
	pub fn get(&self, k: &K) -> Option<Handle<K,V>> {
		let lh = self.lock.read();
		let p = lh.m.get(k).map(|r| r as *const _);
		// SAFE: Lock handle is carried with the pointer, pointer can't be invalidated until that handle is dropped
		p.map(|ptr| unsafe { Handle {
			_ref_handle: lh,
			data_ptr: &*ptr,
			}})
	}
	pub fn take(&self, k: &K) -> Option<V> {
		let mut lh = self.lock.write();
		lh.m.remove(k)
	}
	pub fn insert(&self, k: K, v: V) -> Result<(), V> {
		use ::kernel::lib::collections::vec_map::Entry;
		let mut lh = self.lock.write();
		match lh.m.entry(k)
		{
		Entry::Vacant(e) => { e.insert(v); Ok(()) },
		Entry::Occupied(_) => { Err(v) }
		}
	}
	pub fn replace(&self, k: K, v: V) -> Option<V> {
		use ::kernel::lib::collections::vec_map::Entry;
		let mut lh = self.lock.write();
		match lh.m.entry(k)
		{
		Entry::Vacant(e) => { e.insert(v); None },
		Entry::Occupied(mut e) => { Some( ::core::mem::replace(e.get_mut(), v) ) }
		}
	}

	/// Obtain the outer lock and iterate
	pub fn iter(&self) -> impl Iterator<Item=(&'_ K, &'_ V,)> {
		let lh = self.lock.read();
		Iter {
			// SAFE: This pointer won't outlive the lock handle it came from, and the pointer is stable
			iter: unsafe { (*(&lh.m as *const VecMap<_,_>)).iter() },
			_ref_handle: lh,
		}
	}
}
pub struct Iter<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync>
{
	_ref_handle: rwlock::Read<'a, SharedMapInner<K,V>>,
	iter: ::kernel::lib::collections::vec_map::Iter<'a, K, V>,
}
impl<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync> Iterator for Iter<'a, K, V>
{
	type Item = (&'a K, &'a V,);
	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next()
	}
}

pub struct Handle<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync>
{
	_ref_handle: rwlock::Read<'a, SharedMapInner<K,V>>,
	data_ptr: &'a V,
}
impl<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync> ::core::ops::Deref for Handle<'a, K, V>
{
	type Target = V;
	fn deref(&self) -> &V {
		self.data_ptr
	}
}

