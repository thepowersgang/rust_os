/*
 */
use kernel::prelude::*;
use kernel::metadevs::video;
use interface::Interface;
use queue::{Queue,Buffer};
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use kernel::async::Mutex;

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

struct DeviceCore<I>
where
	I: Interface + Send + Sync
{
	interface: I,
	controlq: Queue,
	cursorq: Queue,

	scanouts: Mutex<Vec<Option<Framebuffer<I>>>>,
}

struct Framebuffer<I>
where
	I: Interface + Send + Sync
{
	dev: ArefBorrow<DeviceCore<I>>,
	/// Handle to video metadev registration
	_video_handle: video::FramebufferRegistration,
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
			});

		let di = core.get_display_info();
		log_debug!("di = {:?}", di);

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
		let hdr = hw::CtrlHeader {
			type_: hw::VIRTIO_GPU_CMD_GET_DISPLAY_INFO as u32,
			flags: hw::VIRTIO_GPU_FLAG_FENCE,
			fence_id: 1,
			ctx_id: 0,
			_padding: 0,
			};
		let mut ret_hdr: hw::CtrlHeader = ::kernel::lib::PodHelpers::zeroed();
		let mut ret_info: [hw::DisplayOne; 16] = ::kernel::lib::PodHelpers::zeroed();
		let h = self.controlq.send_buffers(&self.interface, &mut [
			Buffer::Read(::kernel::lib::as_byte_slice(&hdr)),
			Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_hdr)),
			Buffer::Write(::kernel::lib::as_byte_slice_mut(&mut ret_info)),
			]);
		match h.wait_for_completion()
		{
		Ok(bytes) => todo!("{} bytes from gpu request", bytes),
		Err( () ) => panic!("TODO"),
		}
	}
}

impl<I> video::Framebuffer for Framebuffer<I>
where
	I: 'static + Interface + Send + Sync
{
	fn as_any(&self) -> &Any {
		self as &Any
	}
	fn activate(&mut self) {
		// TODO
	}
	
	fn get_size(&self) -> video::Dims {
		// TODO
		todo!("");
	}
	fn set_size(&mut self, _newsize: video::Dims) -> bool {
		// TODO
		false
	}
	
	fn blit_inner(&mut self, dst: video::Rect, src: video::Rect) {
	}
	fn blit_ext(&mut self, dst: video::Rect, src: video::Rect, srf: &video::Framebuffer) -> bool {
		false
	}
	fn blit_buf(&mut self, dst: video::Rect, buf: &[u32]) {
	}
	fn fill(&mut self, dst: video::Rect, colour: u32) {
	}
	fn move_cursor(&mut self, _p: Option<video::Pos>) {
	}
}


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
	pub struct CtrlHeader
	{
		pub type_: u32,
		pub flags: u32,
		pub fence_id: u64,
		pub ctx_id: u32,
		pub _padding: u32,
	}

	#[repr(C)]
	#[derive(Debug)]
	pub struct Rect
	{
		pub x: u32,
		pub y: u32,
		pub width: u32,
		pub height: u32,
	}
	#[repr(C)]
	#[derive(Debug)]
	pub struct DisplayOne
	{
		pub r: Rect,
		pub enabled: u32,
		pub flags: u32,
	}
}

