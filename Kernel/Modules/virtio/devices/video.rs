//
//
//
/// VirtIO GPU/Video driver
use kernel::prelude::*;
use kernel::metadevs::video;
use crate::interface::Interface;
use crate::queue::{Queue,Buffer};
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use kernel::sync::Mutex;

/// Device instance (as stored by the device manager)
pub struct VideoDevice<I>
where
	I: Interface + Send + Sync
{
	_core: Aref<DeviceCore<I>>
}
impl<I> ::kernel::device_manager::DriverInstance for VideoDevice<I>
where
	I: Interface + Send + Sync
{
}

/// Common device structure ("owned" by the device manager, shared by scanouts)
struct DeviceCore<I>
where
	I: Interface + Send + Sync
{
	interface: I,
	controlq: Queue,
	cursorq: Queue,

	scanouts: Mutex<Vec<Option<video::FramebufferRegistration>>>,
	cursors: Mutex<Vec<Cursor<I>>>,
	next_resource_id: ::core::sync::atomic::AtomicU32,
}

/// Video metadevice framebuffer wrapping a scanout
struct Framebuffer<I>
where
	I: Interface + Send + Sync
{
	scanout_idx: usize,
	dims: (u32, u32,),

	backing_alloc: ::kernel::memory::virt::AllocHandle,
	backing_res: Resource2D<I>,
}

/// Cursor
struct Cursor<I>
where
	I: Interface + Send + Sync
{
	_backing_alloc: ::kernel::memory::virt::AllocHandle,
	backing_res: Resource2D<I>,
	hot_pos: (u32, u32,)
}

/// 2D Resource
struct Resource2D<I>
where
	I: Interface + Send + Sync
{
	dev: ArefBorrow<DeviceCore<I>>,
	idx: u32,
	bpp: u8,
	width: u32,
}

impl<I> VideoDevice<I>
where
	I: 'static + Interface + Send + Sync
{
	pub fn new(mut int: I) -> Self
	{
		// SAFE: Read-only field
		let num_scanouts = unsafe { int.cfg_read_32(8) } as usize;

		let core = Aref::new(DeviceCore {
			controlq: int.get_queue(0, 0).expect("Queue #0 'controlq' missing on virtio gpu device"),
			cursorq: int.get_queue(1, 0).expect("Queue #1 'cursorq' missing on virtio gpu device"),
			scanouts: Mutex::new(Vec::from_fn(num_scanouts, |_| None)),
			interface: int,
			cursors: Mutex::new(Vec::new()),
			next_resource_id: ::core::sync::atomic::AtomicU32::new(1),
			});

		let di = core.get_display_info();
		log_debug!("di = {:?}", di);
		for (i,screen) in Iterator::enumerate( di[..num_scanouts].iter() )
		{
			if screen.enabled != 0
			{
				log_log!("Scanout #{} enabled: {:?} flags={:#x}", i, screen.r, screen.flags);
				core.scanouts.lock()[i] = Some(video::add_output( Box::new(Framebuffer::new(core.borrow(), i, screen)) ));
			}
		}

		// Setup the cursor
		// - Allocate a resource
		// - Populate with a suitable cursor
		{
			let mut cursor_list = core.cursors.lock();
			// Create a standard arrow cursor (TODO: Have a standard set in metadevs::video)
			cursor_list.push( Cursor::new(core.borrow(), &[
				0x7F,0x00,0x00,0x00, 0x00,0x00,0x00,0x00,
				0x7F,0x7F,0x00,0x00, 0x00,0x00,0x00,0x00,
				0x7F,0xFF,0x7F,0x00, 0x00,0x00,0x00,0x00,
				0x7F,0xFF,0xFF,0x7F, 0x00,0x00,0x00,0x00,
				0x7F,0xFF,0xFF,0xFF, 0x7F,0x00,0x00,0x00,
				0x7F,0xFF,0xFF,0xFF, 0xFF,0x7F,0x00,0x00,
				0x7F,0xFF,0xFF,0xFF, 0xFF,0xFF,0x7F,0x00,
				0x7F,0xFF,0xFF,0xFF, 0xFF,0x7F,0x7F,0x7F,

				0x7F,0xFF,0xFF,0xFF, 0x7F,0x00,0x00,0x00,
				0x7F,0xFF,0x7F,0xFF, 0x7F,0x00,0x00,0x00,
				0x7F,0x7F,0x00,0x7F, 0xFF,0x7F,0x00,0x00,
				0x00,0x00,0x00,0x7F, 0xFF,0x7F,0x00,0x00,
				0x00,0x00,0x00,0x7F, 0xFF,0x7F,0x00,0x00,
				0x00,0x00,0x00,0x7F, 0xFF,0xFF,0x7F,0x00,
				0x00,0x00,0x00,0x00, 0x7F,0xFF,0x7F,0x00,
				0x00,0x00,0x00,0x00, 0x7F,0x7F,0x7F,0x00
				], 8,  0,0) );
		}
		// - Update the cursor
		core.set_cursor(/*index=*/0,  /*scanout=*/0, video::Pos::new(!0,!0));

		VideoDevice {
			_core: core,
			}
	}
}

impl<I> DeviceCore<I>
where
	I: Interface + Send + Sync
{

	fn get_display_info(&self) -> /*SmallVec<*/[hw::DisplayOne; 16]//>
	{
		log_trace!("get_display_info()");
		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_GET_DISPLAY_INFO as u32,
			flags: hw::VIRTIO_GPU_FLAG_FENCE,
			fence_id: 1,
			ctx_id: 0,
			_padding: 0,
			};
		let mut ret_hdr: hw::CtrlHeader = ::kernel::lib::PodHelpers::zeroed();
		let mut ret_info: [hw::DisplayOne; 16] = ::kernel::lib::PodHelpers::zeroed();
		let rv = self.controlq.send_buffers_blocking(&self.interface, &mut [
				Buffer::Read(::kernel::lib::as_byte_slice(&hdr)),
				Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_hdr)),
				Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_info)),
			]);
		match rv
		{
		Ok(bytes) => {
			assert_eq!(bytes, ::core::mem::size_of_val(&ret_hdr) + ::core::mem::size_of_val(&ret_info), "Mismatched respose size");
			ret_info
			},
		Err( () ) => panic!("TODO: Handle error waiting for VIRTIO_GPU_CMD_GET_DISPLAY_INFO response"),
		}
	}

	fn allocate_resource_id(&self) -> u32
	{
		self.next_resource_id.fetch_add(1, ::core::sync::atomic::Ordering::SeqCst)
	}
	fn send_cmd<T: kernel::lib::POD + ::core::fmt::Debug>(&self, hdr: &hw::CtrlHeader, cmd: &T) -> hw::CtrlHeader {
		log_trace!("send_cmd(hdr={:?}, cmd={:?}", hdr, cmd);
		self.send_cmd_raw(hdr, ::kernel::lib::as_byte_slice(cmd))
	}
	fn send_cmd_raw(&self, hdr: &hw::CtrlHeader, cmd: &[u8]) -> hw::CtrlHeader {
		let mut ret_hdr: hw::CtrlHeader = ::kernel::lib::PodHelpers::zeroed();
		let _rv = self.controlq.send_buffers_blocking(&self.interface, &mut [
				Buffer::Read(::kernel::lib::as_byte_slice(hdr)),
				Buffer::Read(cmd),
				Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_hdr)),
			]).expect("");
		ret_hdr
	}
	fn allocate_resource(self: &ArefBorrow<Self>, format: hw::virtio_gpu_formats, width: u32, height: u32) -> Resource2D<I>
	{
		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_RESOURCE_CREATE_2D as u32,
			flags: 0,
			fence_id: 0,
			ctx_id: 0,
			_padding: 0,
			};
		let res_id = self.allocate_resource_id();
		let cmd = hw::ResourceCreate2d {
			resource_id: res_id,
			format: format as u32,
			width: width,
			height: height,
			};
		self.send_cmd(&hdr, &cmd);

		Resource2D {
			dev: self.reborrow(),
			bpp: match format
				{
				hw::VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM => 4,
				hw::VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM => 4,
				_ => todo!(""),
				},
			width: width,
			idx: res_id,
			}
	}

	fn set_scanout_backing(&self, scanout_idx: usize, rect: hw::Rect, resource_handle: &Resource2D<I>)
	{
		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_SET_SCANOUT as u32,
			flags: 0,
			fence_id: 0,
			ctx_id: 0,
			_padding: 0,
			};
		let cmd = hw::SetScanout {
			r: rect,
			scanout_id: scanout_idx as u32,
			resource_id: resource_handle.idx,
			};
		let mut ret_hdr: hw::CtrlHeader = ::kernel::lib::PodHelpers::zeroed();

		let _rv = self.controlq.send_buffers_blocking(&self.interface, &mut [
				Buffer::Read(::kernel::lib::as_byte_slice(&hdr)),
				Buffer::Read(::kernel::lib::as_byte_slice(&cmd)),
				Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_hdr)),
			]).expect("");
	}

	fn set_cursor(&self, cursor_index: usize, scanout: usize, pos: video::Pos)
	{
		log_trace!("set_cursor(#{}, {}, {:?})", cursor_index, scanout, pos);
		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_UPDATE_CURSOR as u32,
			flags: 0,
			fence_id: 0,
			ctx_id: 0,
			_padding: 0,
			};
		let cmd = self.cursors.lock()[cursor_index].get_update(scanout, pos);
		let mut ret_hdr: hw::CtrlHeader = ::kernel::lib::PodHelpers::zeroed();

		let _rv = self.cursorq.send_buffers_blocking(&self.interface, &mut [
				Buffer::Read(::kernel::lib::as_byte_slice(&hdr)),
				Buffer::Read(::kernel::lib::as_byte_slice(&cmd)),
				Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_hdr)),
			]).expect("Error setting cursor");
	}
	fn move_cursor(&self, scanout: usize, pos: video::Pos)
	{
		log_trace!("move_cursor({}, {:?})", scanout, pos);
		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_MOVE_CURSOR as u32,
			flags: 0,
			fence_id: 0,
			ctx_id: 0,
			_padding: 0,
			};
		let cmd = hw::UpdateCursor {
			pos: hw::CursorPos {
				scanout_id: scanout as u32,
				x: pos.x,
				y: pos.y,
				_padding: 0,
				},
			resource_id: 1,	// has to be non-zero
			hot_x: 0,
			hot_y: 0,
			_padding: 0,
			};
		let mut ret_hdr: hw::CtrlHeader = ::kernel::lib::PodHelpers::zeroed();

		let _rv = self.cursorq.send_buffers_blocking(&self.interface, &mut [
				Buffer::Read(::kernel::lib::as_byte_slice(&hdr)),
				Buffer::Read(::kernel::lib::as_byte_slice(&cmd)),
				Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_hdr)),
			]).expect("Error setting cursor");
	}
}

impl<I> Framebuffer<I>
where
	I: 'static + Interface + Send + Sync
{
	fn new(dev: ArefBorrow<DeviceCore<I>>, scanout_idx: usize, info: &hw::DisplayOne) -> Self
	{
		let fb = ::kernel::memory::virt::alloc_dma(64, ::kernel::lib::num::div_up(info.r.width as usize * info.r.width as usize * 4, ::kernel::PAGE_SIZE), "virtio-video").expect("");
		// - Create resource (TODO: Should the resource handle its backing buffer?)
		let mut res = dev.allocate_resource(hw::VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM, info.r.width, info.r.height);
		// SAFE: We'e ensuring that both the backing memory and the resource are kept as long as they're in use
		unsafe {
			// - Bind framebuffer to it
			res.attach_backing(fb.as_slice(0,(info.r.width * info.r.height) as usize));
		}
		// - Set scanout's backing to that resource
		dev.set_scanout_backing(scanout_idx, info.r, &res);

		Framebuffer {
			scanout_idx: scanout_idx,
			dims: (info.r.width, info.r.height,),
			backing_alloc: fb,
			backing_res: res,
			}
	}

	fn get_scanline_mut(&mut self, idx: u32) -> &mut [u32] {
		let pitch_bytes = self.dims.0 as usize * 4;
		let row_start = idx as usize * pitch_bytes;
		self.backing_alloc.as_mut_slice(row_start, self.dims.0 as usize)
	}
}
impl<I> video::Framebuffer for Framebuffer<I>
where
	I: 'static + Interface + Send + Sync
{
	fn as_any(&self) -> &dyn Any {
		self as &dyn Any
	}
	fn activate(&mut self) {
		// TODO
	}
	
	fn get_size(&self) -> video::Dims {
		video::Dims {
			w: self.dims.0,
			h: self.dims.1,
			}
	}
	fn set_size(&mut self, _newsize: video::Dims) -> bool {
		// TODO
		false
	}
	
	fn blit_inner(&mut self, _dst: video::Rect, _src: video::Rect) {
		todo!("blit_inner");
	}
	fn blit_ext(&mut self, _dst: video::Rect, _src: video::Rect, _srf: &dyn video::Framebuffer) -> bool {
		false
	}
	fn blit_buf(&mut self, dst: video::Rect, buf: video::StrideBuf<'_, u32>) {
		log_trace!("blit_buf({:?}, len={},{})", dst, buf.stride(), buf.count());
		// Iterate rows of the input
		let src_pitch = dst.w() as usize;
		for (row,src) in kernel::lib::ExactZip::new( dst.top() .. dst.bottom(), buf.chunks(src_pitch) )
		{
			//log_trace!("> {}: {:p}", row, src);
			let out_row = self.get_scanline_mut(row);
			out_row[dst.left() as usize .. dst.right() as usize].copy_from_slice(src);
		}
		// VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D
		self.backing_res.transfer_to_host(dst);
		// VIRTIO_GPU_CMD_RESOURCE_FLUSH
		self.backing_res.flush(dst);
	}
	fn fill(&mut self, dst: video::Rect, colour: u32) {
		todo!("fill({:?}, {:06x})", dst, colour);
	}
	fn move_cursor(&mut self, p: Option<video::Pos>)
	{
		match p
		{
		Some(p) => self.backing_res.dev.move_cursor(self.scanout_idx, p),
		// TODO: Determine if the cursor is being moved between framebuffers?
		None => self.backing_res.dev.move_cursor(self.scanout_idx, video::Pos::new(!0,!0)),
		}
	}
}

impl<I> Cursor<I>
where
	I: 'static + Interface + Send + Sync
{
	fn new(dev: ArefBorrow<DeviceCore<I>>, data: &[u8], width: usize, hot_x: u32, hot_y: u32) -> Self
	{
		const W: u32 = 64;
		const H: u32 = 64;
		let mut fb = ::kernel::memory::virt::alloc_dma(64, ::kernel::lib::num::div_up((W*H*4) as usize, ::kernel::PAGE_SIZE), "virtio-video").expect("");
		for (d,s) in Iterator::zip( fb.as_mut_slice::<u32>(0, (W*H) as usize).chunks_mut(W as usize), data.chunks(width) )
		{
			for (d,s) in Iterator::zip( d.iter_mut(), s.iter() )
			{
				let alpha = (*s & 0x7F) << 1 | (*s & 0x1);
				*d = (alpha as u32) << 24 | (if *s & 0x80 != 0 { 0xFFFFFF } else { 0x000000 });
			}
		}
		let mut res = dev.allocate_resource(hw::VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM, 64, 64);
		//let mut res = dev.allocate_resource(hw::VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM, 64, 64);
		// SAFE: We'e ensuring that both the backing memory and the resource are kept as long as they're in use
		unsafe {
			res.attach_backing(fb.as_slice(0, (W*H) as usize));
			res.transfer_to_host(video::Rect::new(0, 0, W, H));
			res.flush(video::Rect::new(0, 0, W, H));
		}
		Cursor {
			_backing_alloc: fb,
			backing_res: res,
			hot_pos: (hot_x, hot_y,),
			}
	}
}
impl<I> Cursor<I>
where
	I: Interface + Send + Sync
{
	pub fn get_update(&self, scanout: usize, pos: video::Pos) -> hw::UpdateCursor
	{
		hw::UpdateCursor {
			pos: hw::CursorPos {
				scanout_id: scanout as u32,
				x: pos.x,
				y: pos.y,
				_padding: 0,
				},
			resource_id: self.backing_res.idx,
			hot_x: self.hot_pos.0,
			hot_y: self.hot_pos.1,
			_padding: 0,
			}
	}
}

impl<I> Resource2D<I>
where
	I: 'static + Interface + Send + Sync
{
	/// Attach a buffer to this resource
	pub unsafe fn attach_backing(&mut self, buffer: &[u32])
	{
		// 1. Enumerate contigious sections
		let mut entries: [hw::MemEntry; 16] = ::kernel::lib::PodHelpers::zeroed();
		let mut n_ents = 0;
		{
			fn iter_pages(mut base: *const u8, mut len: usize, mut cb: impl FnMut(u64, usize)) {
				use kernel::PAGE_SIZE;
				let base_ofs = (base as usize) % PAGE_SIZE;
				let max_len = PAGE_SIZE - base_ofs;
				if max_len > len {
					cb(kernel::memory::virt::get_phys(base) as u64, len);
					return ;
				}

				cb(kernel::memory::virt::get_phys(base) as u64, max_len);
				len -= max_len;
				base = (base as usize + max_len) as *const u8;
				while len > PAGE_SIZE {
					cb(kernel::memory::virt::get_phys(base) as u64, PAGE_SIZE);
					len -= PAGE_SIZE;
					base = (base as usize + PAGE_SIZE) as *const u8;
				}
				cb(kernel::memory::virt::get_phys(base) as u64, len);
			}
			let mut exp_phys = kernel::memory::virt::get_phys(buffer.as_ptr()) as u64;
			entries[0].addr = exp_phys;
			let mut cur_len = 0;
			iter_pages(buffer.as_ptr() as *const u8, buffer.len() * 4, |phys, len| {
				if phys != exp_phys {
					entries[n_ents].length = cur_len as u32;
					n_ents += 1;
					entries[n_ents].addr = phys;
					cur_len = 0;
				}
				exp_phys = phys + len as u64;
				cur_len += len;
				});
			entries[n_ents].length = cur_len as u32;
			n_ents += 1;
		}
		log_debug!("Resource2D::attach_backing(self=#{}): entries={:?}", self.idx, &entries[..n_ents]);

		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING as u32,
			flags: 0,
			fence_id: 0,
			ctx_id: 0,
			_padding: 0,
			};
		let cmd = hw::ResourceAttachBacking {
			resource_id: self.idx,
			nr_entries: n_ents as u32,
			};

		let mut ret_hdr: hw::CtrlHeader = ::kernel::lib::PodHelpers::zeroed();
		let _rv = self.dev.controlq.send_buffers_blocking(&self.dev.interface, &mut [
				Buffer::Read(::kernel::lib::as_byte_slice(&hdr)),
				Buffer::Read(::kernel::lib::as_byte_slice(&cmd)),
				Buffer::Read(::kernel::lib::as_byte_slice(&entries[..n_ents])),
				Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_hdr)),
			]).expect("");
	}
	pub fn transfer_to_host(&self, rect: video::Rect)
	{
		log_trace!("Resource2D::transfer_to_host(self=#{}, rect={:?})", self.idx, rect);
		self.dev.send_cmd(&hw::CtrlHeader {
				type_: hw::VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D as u32,
				flags: 0,
				fence_id: 0,
				ctx_id: 0,
				_padding: 0,
				},
			&hw::TransferToHost {
				r: rect.into(),
				offset: (rect.x() + rect.y() * self.width) as u64 * self.bpp as u64,
				resource_id: self.idx,
				_padding: 0,
				});
	}
	pub fn flush(&self, rect: video::Rect)
	{
		log_trace!("Resource2D::flush(self=#{}, rect={:?})", self.idx, rect);
		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_RESOURCE_FLUSH as u32,
			flags: 0,
			fence_id: 0,
			ctx_id: 0,
			_padding: 0,
			};
		let cmd = hw::Flush {
			r: rect.into(),
			resource_id: self.idx,
			_padding: 0,
			};
		self.dev.send_cmd(&hdr, &cmd);
	}
}

impl From<video::Rect> for hw::Rect {
	fn from(rect: video::Rect) -> Self {
		hw::Rect {
			x: rect.x(),
			y: rect.y(),
			width: rect.w(),
			height: rect.h(),
			}
	}
}
impl<I> Drop for Resource2D<I>
where
	I: Interface + Send + Sync
{
	fn drop(&mut self)
	{
		// Release the resource
		self.dev.send_cmd(&hw::CtrlHeader {
				type_: hw::VIRTIO_GPU_CMD_RESOURCE_UNREF as u32,
				flags: 0,
				fence_id: 0,
				ctx_id: 0,
				_padding: 0,
				},
			&hw::ResourceUnref {
				resource_id: self.idx,
				_padding: 0,
				});
		self.idx = 0;
	}
}

/// Hardware type/enum definitions
mod hw
{
	#[repr(u32)]
	#[allow(non_camel_case_types)]
	#[allow(dead_code)]
	pub enum CtrlType
	{
		/* 2d commands */
		VIRTIO_GPU_CMD_GET_DISPLAY_INFO = 0x0100,
		VIRTIO_GPU_CMD_RESOURCE_CREATE_2D,
		VIRTIO_GPU_CMD_RESOURCE_UNREF,
		VIRTIO_GPU_CMD_SET_SCANOUT,
		VIRTIO_GPU_CMD_RESOURCE_FLUSH,
		VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D,
		VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING,
		VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING,
		/* cursor commands */
		VIRTIO_GPU_CMD_UPDATE_CURSOR = 0x0300,
		VIRTIO_GPU_CMD_MOVE_CURSOR,
		/* success responses */
		VIRTIO_GPU_RESP_OK_NODATA = 0x1100,
		VIRTIO_GPU_RESP_OK_DISPLAY_INFO,
		/* error responses */
		VIRTIO_GPU_RESP_ERR_UNSPEC = 0x1200,
		VIRTIO_GPU_RESP_ERR_OUT_OF_MEMORY,
		VIRTIO_GPU_RESP_ERR_INVALID_SCANOUT_ID,
		VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID,
		VIRTIO_GPU_RESP_ERR_INVALID_CONTEXT_ID,
		VIRTIO_GPU_RESP_ERR_INVALID_PARAMETER,
	}
	pub use self::CtrlType::*;

	pub const VIRTIO_GPU_FLAG_FENCE: u32 = 1 << 0;

	#[repr(C)]
	#[derive(Debug)]
	pub struct CtrlHeader
	{
		pub type_: u32,
		pub flags: u32,
		pub fence_id: u64,
		pub ctx_id: u32,
		pub _padding: u32,
	}

	#[repr(C)]
	#[derive(Copy,Clone)]
	pub struct Rect
	{
		pub x: u32,
		pub y: u32,
		pub width: u32,
		pub height: u32,
	}
	impl ::core::fmt::Debug for Rect {
		fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
			write!(f, "Rect {{ {},{} {}x{} }}", self.x, self.y, self.width, self.height)
		}
	}
	#[repr(C)]
	#[derive(Debug)]
	pub struct DisplayOne
	{
		pub r: Rect,
		pub enabled: u32,
		pub flags: u32,
	}

	#[repr(C)]
	#[derive(Debug)]
	pub struct ResourceCreate2d
	{
		pub resource_id: u32,
		pub format: u32,
		pub width: u32,
		pub height: u32,
	}
	#[allow(non_camel_case_types,dead_code)]
	#[derive(Copy,Clone,Debug)]
	pub enum virtio_gpu_formats
	{
		VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM  = 1,
		VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM  = 2,
		VIRTIO_GPU_FORMAT_A8R8G8B8_UNORM  = 3,
		VIRTIO_GPU_FORMAT_X8R8G8B8_UNORM  = 4,

		VIRTIO_GPU_FORMAT_R8G8B8A8_UNORM  = 67,
		VIRTIO_GPU_FORMAT_X8B8G8R8_UNORM  = 68,

		VIRTIO_GPU_FORMAT_A8B8G8R8_UNORM  = 121,
		VIRTIO_GPU_FORMAT_R8G8B8X8_UNORM  = 134,
	}
	pub use self::virtio_gpu_formats::*;
	#[repr(C)]
	#[derive(Debug)]
	pub struct ResourceUnref
	{
		pub resource_id: u32,
		pub _padding: u32,
	}

	#[repr(C)]
	#[derive(Debug)]
	pub struct SetScanout
	{
		pub r: Rect,
		pub scanout_id: u32,
		pub resource_id: u32,
	}

	#[repr(C)]
	#[derive(Debug)]
	pub struct ResourceAttachBacking
	{
		pub resource_id: u32,
		pub nr_entries: u32,
	}
	#[repr(C)]
	#[derive(Debug)]
	pub struct MemEntry
	{
		pub addr: u64,
		pub length: u32,
		pub padding: u32,
	}

	#[repr(C)]
	#[derive(Debug)]
	pub struct TransferToHost
	{
		pub r: Rect,
		pub offset: u64,
		pub resource_id: u32,
		pub _padding: u32,
	}

	#[repr(C)]
	#[derive(Debug)]
	pub struct Flush
	{
		pub r: Rect,
		pub resource_id: u32,
		pub _padding: u32,
	}

	#[repr(C)]
	#[derive(Debug)]
	pub struct CursorPos
	{
		pub scanout_id: u32,
		pub x: u32,
		pub y: u32,
		pub _padding: u32,
	}
	#[repr(C)]
	#[derive(Debug)]
	pub struct UpdateCursor
	{
		pub pos: CursorPos,
		pub resource_id: u32,
		pub hot_x: u32,
		pub hot_y: u32,
		pub _padding: u32,
	}
}

