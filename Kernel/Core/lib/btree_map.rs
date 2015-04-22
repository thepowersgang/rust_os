// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/btree_map.rs
//! B-Tree map
//!
//! B-Trees are a more memory/cache efficient version of binary trees, storing up to `b` items
//! per node
use _common::*;

pub struct BTreeMap<K: Ord,V>
{
	root_node: Option< Box< Node<K,V> > >,
	max_node_size: usize,	// aka 'b'
}

struct Node<K:Ord, V>
{
	key: K,
	val: V,
	
	children: Vec< Box<Node<K,V>> >,
	next: Option< Box<Node<K,V>> >,
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
	
	fn get_ptr<Q: ?Sized>(&self, key: &Q) -> Option<*mut V>
	where
		Q: Ord,
		K: ::lib::borrow::Borrow<Q>
	{
		let mut node = match self.root_node { Some(ref v) => v, None => return None };
		loop
		{
			match node.children.binary_search_by(|v| v.key.borrow().cmp(key))
			{
			Ok(idx) => return Some(&node.children[idx].val as *const _ as *mut _),
			Err(idx) => if idx <= node.children.len() {
					node = &node.children[idx];
				} else if let Some(ref n) = node.next {
					node = n;
				}
				else {
					return None;
				},
			}
		}
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

