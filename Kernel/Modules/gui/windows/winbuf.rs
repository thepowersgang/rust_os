// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/windows/winbuf.rs
// - Backing buffer for a window
use kernel::prelude::*;
use super::super::{Dims,Pos,Rect,Colour};
use core::cell::UnsafeCell;

/// Window backing buffer.
///
/// Interior mutable to allow rendering without holding a spinlock (safe, can at worst
/// cause partial updates to be rendered)
///
/// Usecase: Rendering from the logging thread.
pub struct WinBuf
{
	/// Window dimensions
	dims: Dims,
	/// Window backing buffer
	data: UnsafeCell< Vec<u32> >,
}
// SAFE: Multiple &-ptrs are valid (and quite possible)
unsafe impl Sync for WinBuf {}
// SEND: Maintains no external references (Vec is Send)
unsafe impl Send for WinBuf {}
assert_trait!(Vec<u32> : Send);

impl Clone for WinBuf
{
	fn clone(&self) -> WinBuf {
		WinBuf {
			dims: self.dims,
			// SAFE: &ptr means that window will not resize
			data: UnsafeCell::new( unsafe { (*self.data.get()).clone() } ),
		}
	}
}
impl Default for WinBuf
{
	fn default() -> WinBuf {
		WinBuf {
			dims: Default::default(),
			data: UnsafeCell::new( Default::default() ),
		}
	}
}

impl WinBuf
{
	pub fn dims(&self) -> Dims { self.dims }
	
	pub fn resize(&mut self, newsize: Dims)
	{
		let px_count = newsize.width() as usize * newsize.height() as usize;
		log_trace!("WinBuf::resize({:?}) px_count = {}", newsize, px_count);
		let old_w = self.dims.width();
		let new_w = newsize.width();
		self.dims = newsize;
		
		//let val = (self as *const _ as usize & 0xFFFF) as u32 * (256+9);
		let val = 0;
		
		// SAFE: This is the only place where a resize can happen, and self is &mut
		unsafe {
			let buf = &mut *self.data.get();
			if new_w < old_w {
				for r in 0 .. self.dims.height()
				{
					// Copy from the old start to the new start
					let src_ofs = (old_w * r) as usize;
					let dst_ofs = (new_w * r) as usize;
					buf.copy_within(src_ofs .. src_ofs + new_w as usize, dst_ofs);
				}
			}
			buf.resize(px_count, val);
			if new_w > old_w {
				for r in 0 .. self.dims.height()
				{
					// Copy from the old start to the new start
					let src_ofs = (old_w * r) as usize;
					let dst_ofs = (new_w * r) as usize;
					buf.copy_within(src_ofs .. src_ofs + old_w as usize, dst_ofs);
				}
			}
		}
	}
	
	fn slice(&self) -> &[u32] {
		// SAFE: Buffer will not resize, and multiple writers is allowed
		unsafe { &(*self.data.get())[..] }
	}
	fn slice_mut(&self) -> &mut [u32] {
		// TODO: Find some way of ENSURING that LLVM doesn't do something dumb here (like store a pointer in the buffer, and expect it not to change)
		// SAFE: Buffer will not resize, and multiple writers is allowed
		unsafe { &mut (*self.data.get())[..] }
	}
	
	/// Obtain a Range<usize> given a scanline reference
	fn scanline_range(&self, line: usize, ofs: usize, len: usize) -> ::core::ops::Range<usize>
	{
		if self.dims.width() == 0 {
			0 .. 0
		}
		else {
			assert!(ofs < self.dims.width() as usize, "Offset {} outside width of window buffer ({})", ofs, self.dims.width());
			assert!(line < self.dims.h as usize, "Scanline {} outside height of window buffer ({})", line, self.dims.h);
			
			let pitch_32 = self.dims.width() as usize;
			let len = ::core::cmp::min(len, pitch_32 - ofs);
			
			let l_ofs = line * pitch_32;
			
			l_ofs + ofs .. l_ofs + ofs + len
		}
	}
	
	pub fn scanline_rgn(&self, line: usize, ofs: usize, len: usize) -> &[u32]
	{
		&self.slice()[ self.scanline_range(line, ofs, len) ]
	}
	pub fn scanline_rgn_mut(&self, line: usize, ofs: usize, len: usize) -> &mut [u32]
	{
		&mut self.slice_mut()[ self.scanline_range(line, ofs, len) ]
	}

	/// Render this window buffer at the provided position
	pub fn blit(&self, winpos: Pos, rgn: Rect)
	{
		log_trace!("WinBuf::blit(winpos={:?},rgn={:?})", winpos, rgn);

		let pos = Pos::new(
			winpos.x + rgn.left(),
			winpos.y + rgn.top(),
			);
		let buf = ::kernel::metadevs::video::StrideBuf::new(self.slice(), self.dims.width() as usize);
		//log_trace!("> {:p} (base)", buf);
		let buf = buf.offset(rgn.left() as usize, rgn.top() as usize);
		//log_trace!("> {:p} (ofs)", buf);
		let buf = buf.clip(rgn.w() as usize, rgn.h() as usize);
		//log_trace!("> {:p} (clip)", buf);
		::kernel::metadevs::video::write_buf(pos, buf);
	}
	
	pub fn fill_scanline(&self, line: usize, ofs: usize, len: usize, value: Colour)
	{
		if line >= self.dims.height() as usize || ofs >= self.dims.width() as usize {
			return ;
		}
		let rgn = self.scanline_rgn_mut(line, ofs, len);
		//log_debug!("fill_scanline: rgn = {:p}", &rgn[0]);
		for v in rgn.iter_mut()
		{
			*v = value.as_argb32();
		}
	}
	
	pub fn set_scanline(&self, line: usize, ofs: usize, len: usize, data: &[u32])
	{
		if line >= self.dims.height() as usize || ofs >= self.dims.width() as usize {
			return ;
		}
		let rgn = self.scanline_rgn_mut(line, ofs, len);
		
		for (d,s) in rgn.iter_mut().zip( data.iter() )
		{
			*d = *s;
		}
	}
}

