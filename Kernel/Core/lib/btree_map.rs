// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/btree_map.rs
//! B-Tree map
//!
//! B-Trees are a more memory/cache efficient version of binary trees, storing up to `b` items
//! per node
use prelude::*;

pub struct BTreeMap<K: Ord,V>
{
	root_node: Option< Box< Node<K,V> > >,
	max_node_size: usize,	// aka 'b'
}

struct Node<K, V>
{
	values: Vec< Item<K,V> >,
	children: Vec< Node<K,V> >,
}
struct Item<K, V>
{
	key: K,
	val: V,
}

pub enum Entry<'a, K: 'a, V: 'a>
{
	Vacant(VacantEntry<'a,K,V>),
	Occupied(OccupiedEntry<'a,K,V>),
}
pub struct VacantEntry<'a, K:'a,V:'a> {
	root: &'a mut Node<K,V>,
	key: K,
}
pub struct OccupiedEntry<'a, K:'a,V:'a> {
	node: &'a mut Item<K,V>
}

impl<K: Ord,V> BTreeMap<K,V>
{
	pub fn new() -> BTreeMap<K,V> {
		BTreeMap::with_b(8)
	}
	
	pub fn with_b(b: usize) -> Self {
		BTreeMap {
			root_node: None,
			max_node_size: b,
		}
	}
	
	fn rebalance(&mut self)
	{
	}
	
	fn get_ptr<Q: ?Sized>(&self, key: &Q) -> Option<*mut V>
	where
		Q: Ord,
		K: ::lib::borrow::Borrow<Q>
	{
		let mut node = match self.root_node { Some(ref v) => &**v, None => return None };
		loop
		{
			match node.values.binary_search_by(|v| v.key.borrow().cmp(key))
			{
			Ok(idx) => return Some(&node.values[idx].val as *const _ as *mut _),
			Err(idx) => if idx <= node.children.len() {
					node = &node.children[idx];
				}
				else {
					return None;
				},
			}
		}
	}

	pub fn entry(&mut self, key: K) -> Entry<K,V> {
		unimplemented!()
	}
	
	pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
	where
		Q: Ord,
		K: ::lib::borrow::Borrow<Q>
	{
		unsafe { self.get_ptr(key).map(|x| &*x) }
	}
	pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
	where
		Q: Ord,
		K: ::lib::borrow::Borrow<Q>
	{
		unsafe { self.get_ptr(key).map(|x| &mut *x) }
	}
}

impl<K: Ord, V> Default for BTreeMap<K,V>
{
	fn default() -> BTreeMap<K,V> {
		BTreeMap::new()
	}
}

impl<'a,K,V> VacantEntry<'a,K,V> {
	pub fn insert(self, value: V) -> &'a mut V {
		// 1. Allocate a slot (which may require splitting a node and hence rebalancing the tree)
		unimplemented!()
	}
} 

impl<'a,K,V> OccupiedEntry<'a,K,V> {
	pub fn get_mut(&mut self) -> &mut V {
		&mut self.node.val
	}
	pub fn into_mut(self) -> &'a mut V {
		&mut self.node.val
	}
} 

