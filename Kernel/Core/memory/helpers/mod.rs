pub mod descriptor_pool;
pub mod dma_buffer;

pub use self::dma_buffer::DMABuffer;
pub use self::descriptor_pool::DescriptorPool;


/// Returns an iterator of contigious regions of memory
pub fn iter_contiguous_phys(data: &[u8]) -> impl Iterator<Item=(super::PAddr, u32, bool)> + '_
{
	struct V<'a> {
		data: &'a [u8],
		remain: usize,
		ofs: usize,
	}
	impl<'a> ::core::iter::Iterator for V<'a> {
		type Item = (super::PAddr, u32, bool);
		fn next(&mut self) -> Option<Self::Item> {
			use crate::memory::virt::get_phys;
			assert!(self.ofs <= self.data.len(), "{}+{} > {}", self.ofs, self.remain, self.data.len());
			if self.ofs == self.data.len() {
				return None;
			}

			while self.ofs+self.remain < self.data.len()
				&& get_phys(&self.data[self.ofs+self.remain]) == get_phys(self.data.as_ptr()) + self.remain as super::PAddr
			{
				if self.ofs+self.remain + crate::PAGE_SIZE < self.data.len() {
					self.remain = self.data.len() - self.ofs;
				}
				else {
					self.remain += crate::PAGE_SIZE;
				}
			}
			let is_last = self.ofs + self.remain == self.data.len();
			let rv = (get_phys(&self.data[self.ofs]), self.remain as _, is_last,);
			self.ofs += self.remain.min( self.data.len() - self.ofs );
			self.remain = 0;
			Some(rv)
		}
	}
	V {
		data,
		ofs: 0,
		remain: usize::min(0x1000 - (data.as_ptr() as usize & 0xFFF), data.len() ),
	}
}