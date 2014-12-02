// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/hw/vga.rs
// - VGA (and derivative) device driver
//
// TODO: Move this to an external link-time included module
use _common::*;
use metadevs::video::{Framebuffer,Rect};

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
}

static s_vga_pci_driver: VgaPciDriver = VgaPciDriver;
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
	fn bind(&self, bus_dev: &::device_manager::BusDevice) -> Box<::device_manager::DriverInstance+'static>
	{
		panic!("TODO: Handle non-legacy region VGA devices");
		box VgaDevice::new()
	}
	
}

impl VgaDevice
{
	fn new() -> VgaDevice
	{
		VgaDevice {
			video_handle: ::metadevs::video::add_output(box VgaFramebuffer::new(0x3B0)),
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
		VgaFramebuffer {
			io_base: base,
			window: ::memory::virt::map_hw_rw(0xA0000, (0xC0-0xA0), module_path!()).unwrap(),
		}
	}
}
impl ::metadevs::video::Framebuffer for VgaFramebuffer
{
	fn get_size(&self) -> Rect {
		// Uses mode X hackery
		Rect::new( 0,0, 640,480 )
	}
	fn set_size(&self, newsize: Rect) -> bool {
		// Can't change
		false
	}
	
	fn blit_inner(&self, dst: Rect, src: Rect) {
		panic!("TODO: VGA blit_inner");
	}
	fn blit_ext(&self, dst: Rect, src: Rect, srf: &Framebuffer) -> bool {
		//match srf.cast::<VgaFramebuffer>()
		//{
		//Some(s) => panic!("TODO: VGA blit_ext"),
		//None => false,
		//}
		panic!("TODO: VGA blit_ext");
	}
	fn blit_buf(&self, dst: Rect, buf: &[u32]) {
		panic!("TODO: VGA blit_buf");
	}
	fn fill(&self, dst: Rect, colour: u32) {
		panic!("TODO: VGA fill");
	}
}

// vim: ft=rust

