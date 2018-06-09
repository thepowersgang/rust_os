// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/shared_map/lib.rs
//! A key-value map that internally handles synchronisation
//!
//! A wrapper around RwLock<VecMap>
#![no_std]
#![feature(const_fn)]
use kernel::sync::rwlock::{RwLock, self};

#[macro_use]
extern crate kernel;

pub struct SharedMap<K: Send+Sync+Ord,V: Send+Sync>
{
	lock: RwLock<SharedMapInner<K,V,>>,
}
struct SharedMapInner<K: Send+Sync+Ord, V: Send+Sync>
{
	m: ::kernel::lib::collections::VecMap<K,V>,
}

impl<K: Send+Sync+Ord, V: Send+Sync> SharedMap<K,V>
{
	pub const fn new() -> Self {
		SharedMap {
			lock: RwLock::new(SharedMapInner { m: ::kernel::lib::collections::VecMap::new_const() }),
			}
	}
	pub fn get(&self, k: &K) -> Option<Handle<K,V>> {
		todo!("SharedMap::get")
	}
	pub fn take(&self, k: &K) -> Option<V> {
		todo!("SharedMap::take")
	}
	pub fn insert(&self, k: K, v: V) {
		todo!("SharedMap::insert");
	}
}
pub struct Handle<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync>
{
	ref_handle: rwlock::Read<'a, SharedMapInner<K,V>>,
	data_ptr: &'a V,
}
impl<'a, K: 'a + Send+Sync+Ord, V: 'a + Send+Sync> ::core::ops::Deref for Handle<'a, K, V>
{
	type Target = V;
	fn deref(&self) -> &V {
		self.data_ptr
	}
}

