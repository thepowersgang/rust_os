// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/vga.rs
// - VGA (and derivative) device driver
//
// TODO: Move this to an external link-time included module
use _common::*;
use metadevs::video::{Framebuffer,Rect};
use lib::UintBits;

module_define!(VGA, [DeviceManager, Video], init)

struct VgaPciDriver;
//struct VgaStaticDriver;
struct VgaDevice
{
	video_handle: ::metadevs::video::FramebufferRegistration,
}
/**
 * Real device instance (registered with the video manager)
 */
struct VgaFramebuffer
{
	io_base: u16,
	window: ::memory::virt::AllocHandle,
	crtc: CrtcRegs,
}
struct CrtcRegs
{
	regs: [u8, ..0x20],
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

const PIX_PER_CHAR: uint = 16u;

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
			video_handle: ::metadevs::video::add_output(box VgaFramebuffer::new(iobase)),
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
			crtc: CrtcRegs::load(base + 0x24),	// Colour CRTC regs
			};
		
		// 320x240 @60Hz
		rv.set_crtc(CrtcAttrs::from_res(320, 240, 60));
		
		rv
	}
	
	fn set_crtc(&mut self, attrs: CrtcAttrs)
	{
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
	}
}

impl CrtcRegs
{
	fn load(base: u16) -> CrtcRegs
	{
		let mut rv = CrtcRegs {
			regs: [0, ..0x20],
			};
		rv.read(base);
		rv
	}
	
	// CR0: H Total
	pub fn set_h_total(&mut self, val: u16)
	{
		assert!(val <= 0xFF);
		self.regs[0] = (val & 0xFF) as u8;
	}
	// CR1: H Display End
	pub fn set_h_disp_end(&mut self, val: u16)
	{
		assert!(val <= 0xFF);
		self.regs[1] = (val & 0xFF) as u8;
	}
	// CR2: H Blank Start
	pub fn set_h_blank_start(&mut self, val: u16)
	{
		assert!(val <= 0xFF);
		self.regs[2] = (val & 0xFF) as u8;
	}
	// CR3: H Blank Length
	pub fn set_h_blank_len(&mut self, val: u16)
	{
		assert!(val <= 0x3F);
		self.regs[3] &= !(0x1F << 0);
		self.regs[3] |= (val & 0x1F) as u8;
		self.regs[0x5] &= !(1 << 7);
		self.regs[0x5] |= val.bit(5) as u8 << 7;
	}
	// CR4: H Sync Start
	pub fn set_h_sync_start(&mut self, val: u16)
	{
		assert!(val <= 0x1FF);
		self.regs[4] = (val & 0xFF) as u8;
		self.regs[0x1A] &= !(1 << 4);
		self.regs[0x1A] |= val.bit(8) as u8 << 4;
	}
	// CR5: H Sync End
	pub fn set_h_sync_end(&mut self, val: u16)
	{
		assert!(val <= 0x1F);
		self.regs[5] &= !(0x1F);
		self.regs[5] &= (val & 0x1F) as u8;
	}
	// CR6: V Total
	pub fn set_v_total(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[6] = (val & 0xFF) as u8;
		self.regs[7] &= !( (1 << 0) | (1 << 5) );
		self.regs[7] |= (val.bit(8) << 0 | val.bit(9) << 5) as u8;
	}
	// CR12: V Display End
	pub fn set_v_disp_end(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x12] = (val & 0xFF) as u8;
		// CR7[1,6] := val[8,9]
		self.regs[0x07] &= !( 1 << 1 | 1 << 6 );
		self.regs[0x07] |= (val.bit(8) << 1 | val.bit(9) << 6) as u8;
	}
	// CR15: V Blank Start
	pub fn set_v_blank_start(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x15] = (val & 0xFF) as u8;
		self.regs[0x07] &= !( 1 << 3 );
		self.regs[0x07] |= val.bit(8) as u8 << 3;
		self.regs[0x09] &= !( 1 << 5 );
		self.regs[0x09] |= val.bit(9) as u8 << 5;
	}
	// CR16: V Blank End
	pub fn set_v_blank_end(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x16] = (val & 0xFF) as u8;
		self.regs[0x1A] &= !( 3 << 6 );
		self.regs[0x1A] |= val.bits(8,9) as u8 << 6;
	}
	// CR10: V Sync Start
	pub fn set_v_sync_start(&mut self, val: u16)
	{
		assert!(val <= 0x3FF);
		self.regs[0x10] = (val & 0xFF) as u8;
		self.regs[0x07] &= !( 1 << 2 | 1 << 7 );
		self.regs[0x07] |= (val.bit(8) << 2 | val.bit(9) << 7) as u8;
	}
	// CR11: V Sync End
	pub fn set_v_sync_end(&mut self, val: u16)
	{
		assert!(val <= 0xF);
		self.regs[0x11] &= !( 0xF );
		self.regs[0x11] |= (val & 0xF) as u8;
	}
	// CR18: Line Compare - The scanline where ScreenA finishes (And ScreenB starts)
	pub fn set_line_compare(&mut self, val: u16)
	{
		assert!(val < 0x3FF);
		self.regs[0x18] = (val & 0xFF) as u8;
		self.regs[0x07] &= !( 1 << 4 );
		self.regs[0x07] |= val.bit(8) as u8 << 4;
		self.regs[0x09] &= !( 1 << 6 );
		self.regs[0x09] |= val.bit(9) as u8 << 6;
	}
	// CR13: Offset (vertical scrolling)
	pub fn set_offset(&mut self, val: u16)
	{
		assert!(val < 0x1FF);
		self.regs[0x13] = (val & 0xFF) as u8;
		self.regs[0x1B] &= !( 1 << 4 );
		self.regs[0x1B] |= val.bit(8) as u8 << 4;
	}
	
	// CR8: Byte Pan
	pub fn set_byte_pan(&mut self, val: u8)
	{
		self.regs[8] &= !( 3 << 5 );
		self.regs[8] |= (val & 3) << 5
	}

	// CRC/CRD: Screen Start
	pub fn set_screen_start(&mut self, val: u16)
	{
		self.regs[0xC] = (val >> 8) as u8;
		self.regs[0xD] = (val & 0xFF) as u8;
	}
	
	
	fn read(&mut self, base: u16)
	{
		for (idx,val) in self.regs.iter_mut().enumerate()
		{
			unsafe {
				::arch::x86_io::outb(base + 0, idx as u8);
				*val = ::arch::x86_io::inb(base + 1);
			}
		}
	}

	pub fn commit(&mut self, base: u16)
	{
		for (idx,val) in self.regs.iter().enumerate()
		{
			unsafe {
				::arch::x86_io::outb(base + 0, idx as u8);
				::arch::x86_io::outb(base + 1, *val);
			}
		}
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
		// 320x200x 255
		Rect::new( 0,0, 320,200 )
	}
	fn set_size(&self, _newsize: Rect) -> bool {
		// Can't change
		false
	}
	
	fn blit_inner(&self, dst: Rect, src: Rect) {
		panic!("TODO: VGA blit_inner {} to {}", src, dst);
	}
	fn blit_ext(&self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool {
		match srf.downcast_ref::<VgaFramebuffer>()
		{
		Some(_) => panic!("TODO: VGA blit_ext {} to  {}", src, dst),
		None => false,
		}
	}
	fn blit_buf(&self, dst: Rect, buf: &[u32]) {
		panic!("TODO: VGA blit_buf {} pixels to {}", buf.len(), dst);
	}
	fn fill(&self, dst: Rect, colour: u32) {
		panic!("TODO: VGA fill {} with {:06x}", dst, colour);
	}
}

// vim: ft=rust

