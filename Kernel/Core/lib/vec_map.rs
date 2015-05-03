// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/vec_map.rs
//! Sorted vector backed Key-Value map.
use prelude::*;
use lib::borrow::Borrow;

/// Primitive key-value map backed by a sorted vector
pub struct VecMap<K: Ord,V>
{
	ents: Vec<(K,V)>,
}
/// Immutable iterator for VecMap
pub struct Iter<'a, K: 'a, V: 'a>
{
	pos: usize,
	ents: &'a [(K,V)],
}

/// An entry in a VecMap
pub enum Entry<'a, K: 'a + Ord, V: 'a>
{
	Occupied(OccupiedEntry<'a, K, V>),
	Vacant(VacantEntry<'a, K, V>),
}
/// An occupied entry in a VecMap
pub struct OccupiedEntry<'a, K: 'a + Ord, V: 'a>
{
	map: &'a mut VecMap<K,V>,
	slot: usize,
}
/// An unoccupied entyr in a VecMap
pub struct VacantEntry<'a, K: 'a + Ord, V: 'a>
{
	map: &'a mut VecMap<K,V>,
	slot: usize,
	key: K,
}

impl<K: Ord, V> VecMap<K,V>
{
	/// Create a new (empty) VecMap
	pub fn new() -> VecMap<K,V> {
		VecMap {
			ents: Vec::new(),
		}
	}
	
	/// Returns the previous item (replaced), if any
	pub fn insert(&mut self, key: K, value: V) -> Option<V> {
		match self.entry(key)
		{
		Entry::Occupied(e) => {
			Some( ::core::mem::replace(e.into_mut(), value) )
			},
		Entry::Vacant(e) => {
			e.insert(value);
			None
			},
		}
	}
	
	/// Return an 'entry' in the map, allowing cheap handling of insertion/lookup
	pub fn entry(&mut self, key: K) -> Entry<K, V>
	{
		// Binary search for the specified key
		match self.ents.binary_search_by(|e| e.0.cmp(&key))
		{
		Ok(idx) =>  Entry::Occupied( OccupiedEntry { map: self, slot: idx } ),
		Err(idx) => Entry::Vacant(   VacantEntry { map: self, slot: idx, key: key } ),
		}
	}

	pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
	where
		Q: Ord,
		K: Borrow<Q>
	{
		match self.ents.binary_search_by(|e| e.0.borrow().cmp(key))
		{
		Ok(idx) => Some( &self.ents[idx].1 ),
		Err(_) => None,
		}
	}
		
	/// Return a read-only iterator
	pub fn iter(&self) -> Iter<K,V> {
		Iter {
			pos: 0,
			ents: &*self.ents,
		}
	}
}

impl<'a, K, V> ::core::iter::Iterator for Iter<'a, K, V>
{
	type Item = (&'a K, &'a V);
	
	fn next(&mut self) -> Option<(&'a K, &'a V)>
	{
		if self.pos < self.ents.len()
		{
			let e = &self.ents[self.pos];
			self.pos += 1;
			Some( (&e.0, &e.1) )
		}
		else
		{
			None
		}
	}
}

impl<'a,K: Ord,V> OccupiedEntry<'a, K, V>
{
	/// Return a limited-lifetime pointer to the item
	pub fn get_mut(&mut self) -> &mut V
	{
		&mut self.map.ents[self.slot].1
	}
	/// Consume the Entry and return a pointer to the item
	pub fn into_mut(self) -> &'a mut V
	{
		&mut self.map.ents[self.slot].1
	}
}

impl<'a,K: Ord,V> VacantEntry<'a, K, V>
{
	/// Insert a value at this position and return a pointer to it
	pub fn insert(self, value: V) -> &'a mut V {
		self.map.ents.insert(self.slot, (self.key, value));
		&mut self.map.ents[self.slot].1
	}
}

