// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/vga.rs
// - VGA (and derivative) device driver
//
// TODO: Move this to an external link-time included module
use _common::*;
use metadevs::video::{Framebuffer,Rect};

module_define!{VGA, [DeviceManager, Video], init}

mod crtc;

struct VgaPciDriver;
//struct VgaStaticDriver;
struct VgaDevice
{
	/// Handle to video metadev registration
	_video_handle: ::metadevs::video::FramebufferRegistration,
}
/**
 * Real device instance (registered with the video manager)
 */
struct VgaFramebuffer
{
	io_base: u16,
	window: ::memory::virt::AllocHandle,
	crtc: crtc::CrtcRegs,
	w: uint,
	h: uint,
}
struct CrtcAttrs
{
	frequency: u16,
	h_front_porch: u16,
	h_active: u16,
	h_back_porch: u16,
	h_sync_len: u16,
	
	v_front_porch: u16,
	v_active: u16,
	v_back_porch: u16,
	v_sync_len: u16,
}

#[allow(non_upper_case_globals)]
static s_vga_pci_driver: VgaPciDriver = VgaPciDriver;
#[allow(non_upper_case_globals)]
static s_legacy_bound: ::core::atomic::AtomicBool = ::core::atomic::INIT_ATOMIC_BOOL;

fn init()
{
	// 1. Register Driver
	::device_manager::register_driver(&s_vga_pci_driver);
}

impl ::device_manager::Driver for VgaPciDriver
{
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &::device_manager::BusDevice) -> uint
	{
		let classcode = bus_dev.get_attr("class");
		if classcode & 0xFFFFFF00 == 0x030000 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, _bus_dev: &::device_manager::BusDevice) -> Box<::device_manager::DriverInstance+'static>
	{
		if s_legacy_bound.swap(true, ::core::atomic::Ordering::AcqRel)
		{
			panic!("Duplicate binding of legacy VGA");
		}
		box VgaDevice::new(0x3B0)
	}
	
}

impl VgaDevice
{
	fn new(iobase: u16) -> VgaDevice
	{
		VgaDevice {
			_video_handle: ::metadevs::video::add_output(box VgaFramebuffer::new(iobase)),
		}
	}
}

impl ::device_manager::DriverInstance for VgaDevice
{
}

impl VgaFramebuffer
{
	fn new(base: u16) -> VgaFramebuffer
	{
		// TODO: Modeset VGA into desired mode
		let mut rv = VgaFramebuffer {
			io_base: base,
			window: ::memory::virt::map_hw_rw(0xA0000, (0xC0-0xA0), module_path!()).unwrap(),
			crtc: crtc::CrtcRegs::load(base + 0x24),	// Colour CRTC regs
			w: 320,
			h: 240,
			};
		
		// 320x240 @60Hz
		rv.set_crtc(CrtcAttrs::from_res(320, 240, 60));
		
		rv
	}
	
	fn set_crtc(&mut self, attrs: CrtcAttrs)
	{
		use self::crtc::PIX_PER_CHAR;
		self.crtc.set_line_compare(0);	// Entire screen is Screen B
		self.crtc.set_screen_start(0);	// Screen A starts at offset 0 (making A and B functionally equal)
		self.crtc.set_byte_pan(0);	// Byte pan = 0, no horizontal offset
		
		self.crtc.set_offset(attrs.h_active * 1);	// Aka pitch (TODO: Need the BPP setting)
		
		let htotal = (attrs.h_front_porch + attrs.h_active + attrs.h_back_porch) / PIX_PER_CHAR as u16;
		self.crtc.set_h_total(htotal);
		let h_disp_end = attrs.h_active / PIX_PER_CHAR as u16;
		self.crtc.set_h_disp_end(h_disp_end);
		self.crtc.set_h_blank_start(h_disp_end+1);	// Must be larger than h_disp_end
		let h_blank_len = (attrs.h_front_porch + attrs.h_back_porch) / PIX_PER_CHAR as u16;
		self.crtc.set_h_blank_len( h_blank_len );
		let h_sync_start = (attrs.h_active + attrs.h_front_porch) / PIX_PER_CHAR as u16;
		self.crtc.set_h_sync_start(h_sync_start);
		let h_sync_end = ((h_sync_start + attrs.h_sync_len) / PIX_PER_CHAR as u16) & 31;
		self.crtc.set_h_sync_end(h_sync_end);
		
		// Vertical
		let v_total = attrs.v_front_porch + attrs.v_active + attrs.v_back_porch;
		self.crtc.set_v_total(v_total);
		let v_disp_end = attrs.v_active;
		self.crtc.set_v_disp_end(v_disp_end);
		self.crtc.set_v_blank_start(v_disp_end+1);
		let v_blank_end = attrs.v_front_porch + attrs.v_back_porch;
		self.crtc.set_v_blank_end(v_blank_end);
		let v_sync_start = attrs.v_active + attrs.v_back_porch;
		self.crtc.set_v_sync_start(v_sync_start);
		let v_sync_end = (v_sync_start + attrs.v_sync_len) & 31;
		self.crtc.set_v_sync_end(v_sync_end);
		
		self.crtc.commit(self.io_base + 0x24);
		
		panic!("TODO: Set/check firequency {}Hz", attrs.frequency);
	}
	
	fn col32_to_u8(&self, colour: u32) -> u8
	{
		// 8:8:8 RGB -> 2:3:3 RGB
		let r8 = (colour >> 16) as u8;
		let g8 = (colour >>  8) as u8;
		let b8 = (colour >>  0) as u8;
		
		let r2 = (r8 + 0x3F) >> (8-2);
		let g3 = (g8 + 0x1F) >> (8-3);
		let b3 = (b8 + 0x1F) >> (8-3);
		return (r2 << 6) | (g3 << 3) | (b3 << 0);
	}
}

impl CrtcAttrs
{
	pub fn from_res(w: u16, h: u16, freq: u16) -> CrtcAttrs
	{
		panic!("TODO: Obtain CRTC attributes from resolution {}x{} at {}Hz", w, h, freq);
	}
}

impl ::metadevs::video::Framebuffer for VgaFramebuffer
{
	fn get_size(&self) -> Rect {
		// 320x200x 8bpp
		Rect::new( 0,0, self.w as u16, self.h as u16 )
	}
	fn set_size(&mut self, _newsize: Rect) -> bool {
		// Can't change
		false
	}
	
	fn blit_inner(&mut self, dst: Rect, src: Rect) {
		panic!("TODO: VGA blit_inner {} to {}", src, dst);
	}
	fn blit_ext(&mut self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool {
		match srf.downcast_ref::<VgaFramebuffer>()
		{
		Some(_) => panic!("TODO: VGA blit_ext {} to  {}", src, dst),
		None => false,
		}
	}
	fn blit_buf(&mut self, dst: Rect, buf: &[u32]) {
		panic!("TODO: VGA blit_buf {} pixels to {}", buf.len(), dst);
	}
	fn fill(&mut self, dst: Rect, colour: u32) {
		assert!( dst.within(self.w as u16, self.h as u16) );
		let colour_val = self.col32_to_u8(colour);
		for row in range(dst.y, dst.y + dst.h)
		{
			let scanline = self.window.as_mut_slice::<u8>(row as uint * self.w, dst.w as uint);
			for col in range(dst.x, dst.x + dst.w) {
				scanline[col as uint] = colour_val;
			}
		}
	}
}

// vim: ft=rust

