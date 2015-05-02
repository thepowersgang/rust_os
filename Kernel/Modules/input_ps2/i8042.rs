// "Tifflin" Kernel - ATA Driver
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/i8042.rs
//! x86 PS/2 controller (intel 8042)
use kernel::_common::*;

use kernel::arch::{acpi,x86_io};
use kernel::irqs;

#[derive(Default)]
struct Port(super::PS2Dev);

#[derive(Default)]
struct Ctrlr8042
{
	port1: Option< irqs::ObjectHandle<Port> >,
	port2: Option< irqs::ObjectHandle<Port> >,
}

pub fn init()
{
	// 1. Check with ACPI is this machine has a PS2 controller
	let enabled = if let Some(fadt) = acpi::find::<acpi::Fadt>("FACP", 0) {
			use ::kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
			unsafe { ::kernel::logging::hex_dump("FADT", fadt.data_byte_slice()) };
			//let boot_architecture_flags = fadt.data().boot_architecture_flags;
			assert_eq!( ::core::mem::size_of::<acpi::Fadt>(), 116-36);
			let boot_architecture_flags = unsafe { (&fadt.data_byte_slice()[109-36..]).read_u16::<LittleEndian>().unwrap() };
			log_trace!("FADT boot_architecture_flags = {:#x}", boot_architecture_flags);
			boot_architecture_flags & 2 != 0
		}
		else {
			true
		};
	
	if enabled {
		unsafe {
			let c = Ctrlr8042::new().unwrap();
		}
	}
	else {
		log_log!("8042 PS/2 Controller disabled due to ACPI");
	}
}

impl irqs::Handler for Port
{
	fn handle(&mut self) -> bool {
		unsafe {
			if x86_io::inb(0x64) & 2 != 0 {
				let b = x86_io::inb(0x60);
				log_trace!("PS2 Byte {:#x}", b);
				true
			}
			else {
				false
			}
		}
	}
}

impl Ctrlr8042
{
	unsafe fn new() -> Result<Ctrlr8042,()> {
		let mut ctrlr = Ctrlr8042::default();
		// 1. Disable the controller during setup
		ctrlr.write_cmd(0xAD);	// Disable primary channel
		ctrlr.write_cmd(0xA7);	// Disable secondary channel (ignored if none)
		// - Flush the input FIFO
		while ctrlr.poll_in() {
			ctrlr.read_data();
		}
		
		// Read, Modify, Write the controller's config
		ctrlr.write_cmd(0x20);
		let mut config = ctrlr.read_data().ok().expect("Timeout reading PS/2 config");
		config &= (1<<0)|(1<<1)|(1<<6);
		let can_have_second_port = (config & (1<<5) != 0);
		ctrlr.write_cmd(0x60);
		ctrlr.write_data(config);
		
		// Self-test
		ctrlr.write_cmd(0xAA);
		match ctrlr.read_data() {
		Ok(0x55) => {},
		Ok(v) => panic!("PS/2 self-test failed ({:#x} exp 0x55)", v),
		Err(_) => panic!("Timeout waiting for PS/2 self-test"),
		}
		
		let has_second_port = if can_have_second_port {
				ctrlr.write_cmd(0xA8);	// Enable second port
				ctrlr.write_cmd(0x20);
				let config = ctrlr.read_data().ok().expect("Timeout reading PS/2 config (2)");
				ctrlr.write_cmd(0xA7);	// Disable secondary channel (ignored if none)
				config & (1 << 5) != 0
			}
			else {
				false
			};
		
		// - Flush the input FIFO (again)
		//  > Just in case data arrived while twiddling with ports
		while ctrlr.poll_in() {
			ctrlr.read_data();
		}
		
		let port1_works = {
			ctrlr.write_cmd(0xAB);
			ctrlr.read_data().unwrap() == 0x00
			};
		let port2_works = if has_second_port {
				ctrlr.write_cmd(0xA9);
				ctrlr.read_data().unwrap() == 0x00
			} else {
				false
			};
		
		if !port1_works && !port2_works {
			// nothing works, give up
			todo!("Handle no ports working");
		}
		
		// Enable working ports.
		// - Enable interrupts first
		ctrlr.write_cmd(0x20);
		let mut config = ctrlr.read_data().ok().expect("Timeout reading PS/2 config (2)");
		if port1_works {
			config |= 1 << 0;	// Enable interrupt
		}
		if port2_works {
			config |= 1 << 1;	// Enable interrupt
		}
		log_trace!("config = {:#x}", config);
		ctrlr.write_cmd(0x60);
		ctrlr.write_data(config);
		// - Enable ports second
		if port1_works {
			ctrlr.port1 = Some( ::kernel::irqs::bind_object(1, Box::new(Port::default())) );
			ctrlr.write_cmd(0xAE);
		}
		if port2_works {
			ctrlr.port2 = Some( ::kernel::irqs::bind_object(12, Box::new(Port::default())) );
			ctrlr.write_cmd(0xA8);
		}
		
		Ok( ctrlr )
	}
	
	/// true if write is possible
	unsafe fn poll_out(&mut self) -> bool {
		x86_io::inb(0x64) & 2 == 0
	}
	/// true if read is possible
	unsafe fn poll_in(&mut self) -> bool {
		x86_io::inb(0x64) & 2 != 0
	}
	
	unsafe fn wait_out(&mut self) -> Result<(),()> {
		const MAX_SPINS: usize = 1000;
		let mut spin_count = 0;
		while !self.poll_out() {
			spin_count += 1;
			if spin_count == MAX_SPINS {
				return Err( () );
			}
		}
		Ok( () )
	}
	unsafe fn wait_in(&mut self) -> Result<(),()> {
		const MAX_SPINS: usize = 1000;
		let mut spin_count = 0;
		while !self.poll_in() {
			spin_count += 1;
			if spin_count == MAX_SPINS {
				return Err( () );
			}
		}
		Ok( () )
	}
	
	unsafe fn write_cmd(&mut self, byte: u8) {
		if let Err(_) = self.wait_out() {
			todo!("Handle over-spinning in PS2 controller write");
		}
		x86_io::outb(0x64, byte);
	}
	unsafe fn write_data(&mut self, byte: u8) {
		if let Err(_) = self.wait_out() {
			todo!("Handle over-spinning in PS2 controller write");
		}
		x86_io::outb(0x60, byte);
	}
	unsafe fn read_data(&mut self) -> Result<u8,()> {
		try!( self.wait_in() );
		Ok( x86_io::inb(0x60) )
	}
}


