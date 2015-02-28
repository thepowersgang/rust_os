
use _common::*;

pub struct VecMap<K: Ord,V>
{
	ents: Vec<(K,V)>,
}
pub struct Iter<'a, K: 'a, V: 'a>
{
	pos: usize,
	ents: &'a [(K,V)],
}

pub enum Entry<'a, K: 'a + Ord, V: 'a>
{
	Occupied(OccupiedEntry<'a, K, V>),
	Vacant(VacantEntry<'a, K, V>),
}
pub struct OccupiedEntry<'a, K: 'a + Ord, V: 'a>
{
	map: &'a mut VecMap<K,V>,
	slot: usize,
}
pub struct VacantEntry<'a, K: 'a + Ord, V: 'a>
{
	map: &'a mut VecMap<K,V>,
	slot: usize,
	key: K,
}

impl<K: Ord, V> VecMap<K,V>
{
	pub fn new() -> VecMap<K,V> {
		VecMap {
			ents: Vec::new(),
		}
	}
	
	/// Returns the previous item (replaced), if any
	pub fn insert(&mut self, key: K, value: V) -> Option<V> {
		unimplemented!();
	}
	
	pub fn entry(&mut self, key: K) -> Entry<K, V> {
		// Binary search for the specified key
		match self.ents.binary_search_by(|e| e.0.cmp(&key))
		{
		Ok(idx) =>  Entry::Occupied( OccupiedEntry { map: self, slot: idx } ),
		Err(idx) => Entry::Vacant(   VacantEntry { map: self, slot: idx, key: key } ),
		}
	}
	
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
		if self.pos <= self.ents.len()
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
	pub fn get_mut(&mut self) -> &mut V
	{
		&mut self.map.ents[self.slot].1
	}
	pub fn into_mut(self) -> &'a mut V
	{
		&mut self.map.ents[self.slot].1
	}
}

impl<'a,K: Ord,V> VacantEntry<'a, K, V>
{
	pub fn insert(self, value: V) -> &'a mut V {
		self.map.ents.insert(self.slot, (self.key, value));
		&mut self.map.ents[self.slot].1
	}
}

