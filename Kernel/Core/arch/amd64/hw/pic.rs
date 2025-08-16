
//! Legacy 8259 PIC
use crate::arch::x86_io::{outb,inb};

const CTRL_MASTER: Controller = Controller(0x20);
const CTRL_SLAVE: Controller = Controller(0xA0);
const BASE_ISR: u8 = 0x30;
pub(super) fn disable()
{
	// Disable legacy PIC by masking all interrupts off
	CTRL_SLAVE.ocw1(0xFF);
	CTRL_MASTER.ocw1(0xFF);
}
pub(super) fn register(global_num: usize, callback: super::apic::IRQHandler, info: *const() ) -> Result<(),super::apic::IrqError>
{
	if global_num >= 16 {
		return Err(super::apic::IrqError::BadIndex);
	}
	*HANDLERS[global_num].lock() = Some(Handler { callback, info });
	Ok(())
}
pub fn unregister(global_num: usize) {
	if global_num < 16 {
		*HANDLERS[global_num].lock() = None;
	}
}

static HW_LOCK: crate::sync::Spinlock<()> = crate::sync::Spinlock::new(());

#[derive(Copy,Clone,PartialEq)]
#[derive(Debug)]
pub struct Status {
	pub irr: u8,
	pub isr: u8,
}
pub fn read_status() -> [Status; 2] {
	let _lh = HW_LOCK.lock();
	// SAFE: Locked access
	unsafe {
		[
			CTRL_MASTER.read_status(),
			CTRL_SLAVE .read_status(),
		]
	}
}

/// UNSAFE: Unsynchronised
pub(super) unsafe fn init()
{
	// Bind the fixed interrupt assignments
	bind();
	// Mask all interrupts off, while we reconfigure the controllers
	
	CTRL_MASTER.ocw1(0xFF);
	CTRL_SLAVE.ocw1(0xFF);
	// NOTE: ICW2: ISR numbers (NOTE: These have to be on multiples of 8, as the bottom three bits are unused in 8086 mode)
	// NOTE: ICW3: On the master, this is the bitmask of lines with slaves. On a slsave, this is the master line number
	let master_slave_num = 2;
	CTRL_MASTER.init(BASE_ISR+0, 1 << master_slave_num, Some(0x01));
	CTRL_SLAVE .init(BASE_ISR+8, master_slave_num, Some(0x01));
	// Ensure that there are no pending interrupts from before the remapping
	CTRL_SLAVE .eoi();
	CTRL_MASTER.eoi();
	
	// Unmask all bits
	CTRL_MASTER.ocw1(0);
	CTRL_SLAVE .ocw1(0);
}
fn eoi(is_slave: bool) {
	//log_debug!("EOI {}", is_slave);
	if is_slave {
		CTRL_SLAVE.eoi();
	}
	CTRL_MASTER.eoi();
}

struct Controller(u16);
impl Controller {
	/// Initialise the controller: Send ICW*
	/// 
	/// UNSAFE: Is a sequence of writes, so could race
	unsafe fn init(&self, icw2: u8, icw3: u8, icw4: Option<u8>) {
		outb(self.0 + 0, 0x11 | icw4.is_some() as u8);
		outb(self.0 + 1, icw2);
		outb(self.0 + 1, icw3);
		if let Some(icw4) = icw4 {
			outb(self.0 + 1, icw4);
		}
	}
	/// OCW1: IRQ Mask
	fn ocw1(&self, mask: u8) {
		// SAFE: Write to this register always writes to the mask, so no races possible
		unsafe { outb(self.0 + 1, mask); }
	}
	/// OCW2: Interrupt ACKs
	// Bit 4 clear and bit 3 clear
	fn ocw2(&self, cmd: Ocw2Cmd, line: u8) {
		// SAFE: Writing this value has no side-effects
		unsafe {
			outb(self.0 + 0,
				(0b00<<3)
				| (cmd as u8) << 5
				| (line & 0x7)
				);
		}
	}
	/// OCW3: Misc
	///
	/// UNSAFE: if `read` is set, it expects a read from address 0 - and thus could interleave
	// Bit 4 clear and bit 3 set
	unsafe fn ocw3(&self, smm: Option<bool>, read: Option<Ocw3Read>) {
		outb(self.0 + 0,
			(1<<3)
			| (smm.map(|v| 2|v as u8).unwrap_or(0) << 5)
			| match read {
				None => 0,
				Some(Ocw3Read::Poll) => 1 << 2,
				Some(Ocw3Read::Irr) => 2,
				Some(Ocw3Read::Isr) => 3,
				}
			);
	}

	/// Read the "Interrupt Request" register
	///
	/// UNSAFE: No synchroniation, could interleave writes and reads
	unsafe fn read_irr(&self) -> u8 {
		self.ocw3(None, Some(Ocw3Read::Irr));
		inb(self.0+0)
	}
	/// Read the "Interrupt Status" register
	/// 
	/// UNSAFE: No synchroniation, could interleave writes and reads
	unsafe fn read_isr(&self) -> u8 {
		self.ocw3(None, Some(Ocw3Read::Isr));
		inb(self.0+0)
	}
	/// Read the current interrupt level, clearing any pending interrupt
	/// 
	/// If bit 7 (0x80) is set, an interrupt is pending
	/// 
	/// UNSAFE: No synchroniation, could interleave writes and reads
	unsafe fn poll(&self) -> u8 {
		self.ocw3(None, Some(Ocw3Read::Poll));
		inb(self.0+0)
	}

	/// Request a non-specific end-of-interrupt
	fn eoi(&self) {
		self.ocw2(Ocw2Cmd::EoiNonSpec, 0);
	}

	/// Read both status registers
	///
	/// UNSAFE: No synchronisation of the reads
	unsafe fn read_status(&self) -> Status {
		Status {
			irr: self.read_irr(),
			isr: self.read_isr(),
		}
	}
}

#[allow(dead_code)]
#[repr(u8)]
enum Ocw2Cmd {
	EoiNonSpec = 1,
	EoiSpec = 3,
	RotateNonSpec = 5,
	RotateAutoSet = 4,
	RotateAutoClear = 0,
	RotateSpec = 7,
	SetPrio = 6,
	Nop = 2,
}
enum Ocw3Read {
	Poll,
	Irr,
	Isr,
}

macro_rules! def_handlers {
	( $($i:expr => $name:ident,)* ) => {
		$(
			extern "C" fn $name(_num: usize, _arg1: *const (), _arg2: usize) {
				match HANDLERS[$i].try_lock_cpu() {
				None => {},
				Some(v) => match *v
					{
					None => {},
					Some(ref v) => (v.callback)(v.info),
					},
				}
				eoi($i >= 8);
			}
		)*
		fn bind() {
			use crate::arch::amd64::interrupts::bind_isr;
			$(
			::core::mem::forget(bind_isr(BASE_ISR + $i, $name, 0 as _ , $i).unwrap());
			)*
		}
	}
}
def_handlers! {
	0 => handler_m0,
	1 => handler_m1,
	2 => handler_m2,
	3 => handler_m3,
	4 => handler_m4,
	5 => handler_m5,
	6 => handler_m6,
	7 => handler_m7,
	 8 => handler_s0,
	 9 => handler_s1,
	10 => handler_s2,
	11 => handler_s3,
	12 => handler_s4,
	13 => handler_s5,
	14 => handler_s6,
	15 => handler_s7,
}
struct Handler {
	callback: super::apic::IRQHandler,
	info: *const (),
}
unsafe impl Sync for Handler {}
unsafe impl Send for Handler {}
static HANDLERS: [crate::sync::Spinlock<Option<Handler>>; 16] = [
	const { crate::sync::Spinlock::new(None) }; 16
];
