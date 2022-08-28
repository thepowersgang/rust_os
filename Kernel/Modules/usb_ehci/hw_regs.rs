//! Hardware definitions (register file and constants)
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

pub struct Regs
{
	base: ::kernel::device_manager::IOBinding,
	cap_length: usize,
}
impl Regs
{
	/// UNSAFE: Caller must ensure that the IO binding is a EHCI IO binding
	pub unsafe fn new(h: ::kernel::device_manager::IOBinding) -> Self {
		let cap_length = h.read_8(0);
		Self {
			base: h,
			cap_length: cap_length as usize,
		}
	}

	pub fn get_inner(&self) -> &::kernel::device_manager::IOBinding {
		&self.base
	}
}
/// Capability registers, read-only
impl Regs
{
	pub fn hci_version(&self) -> u16 {
		// SAFE: Reading is safe
		unsafe { self.base.read_16(2) }
	}
	/// Structural Parameters
	pub fn hcs_params(&self) -> u32 {
		// SAFE: Reading is safe
		unsafe { self.base.read_32(4) }
	}
	/// Capability Parameters
	pub fn hcc_params(&self) -> u32 {
		// SAFE: Reading is safe
		unsafe { self.base.read_32(8) }
	}
	/// Companion Port Route Description
	pub fn hcs_port_route(&self) -> u64 {
		// SAFE: Reading is safe
		unsafe { self.base.read_32(12) as u64 | (self.base.read_32(12+4) as u64) << 32 }
	}
}

#[repr(usize)]
pub enum OpReg {
	/// USB Command Register
	/// 
	/// *  0    = Run/Stop (Stop, Run)
	/// *  1    = Host Controller Reset
	/// *  2: 3 = Frame List Size (1024 entries, 512, 256, Reserved)
	/// *  4    = Periodic Schedule Enable
	/// *  5    = Asynchronous Schedule Enable
	/// *  6    = Interrupt on Async Advance Doorbell
	/// *  7    = Light Host Controller Reset
	/// *  8: 9 = Asynchronous Schedule Park Mode Count
	/// * 10    = Reserved (ZERO)
	/// * 11    = Asynchronous Schedule Park Mode Enable
	/// * 12:15 = Reserved (ZERO)
	/// * 16:23 = Interrupt Threshold Control
	/// * 31:24 = Reserved (ZERO)
	UsbCmd,
	/// USB Status Register
	UsbSts,
	/// USB Interrupt Enable Register
	UsbIntr,
	/// Current microframe number (14 bits)
	FrIndex,
	/// Control Data Structure Segment Register
	/// 
	/// Most significant 32-bits of all addresses (only used if "64-bit addressing capability" is set)
	CtrlDsSegment,
	/// Periodic Frame List Base Address Register
	PeriodicListBase,
	/// Current Asynchronous List Address Register
	/// 
	/// - This is updated by the hardware when the list advances.
	/// - Should only be written by software when the list is disabled (see `UsbCmd` bit 5 and `USBSTS_AsyncEnabled`)
	AsyncListAddr,
	/// Configure Flag Register
	ConfigFlag = 0x40 / 4,
	/// Port Status and Control Register (one per port)
	/// NOTE: Use the [Regs::read_port_sc] and [Regs::write_port_sc] functions to access
	PortSc0,
}

/// Operational Registers
impl Regs
{
	pub fn read_op(&self, reg: OpReg) -> u32 {
		// SAFE: Reading any operational register is safe
		unsafe { self.base.read_32(self.cap_length + reg as usize * 4) }
	}
	pub unsafe fn write_op(&self, reg: OpReg, v: u32) {
		#[cfg(debug_assertions)]
		match reg
		{
		OpReg::UsbCmd => assert!(v & 0xFF00_F400 == 0, "Reserved bits set in UsbCmd"),
		OpReg::UsbSts => {},
		OpReg::UsbIntr => {},
		OpReg::FrIndex => panic!("Writing to FrIndex"),
		OpReg::CtrlDsSegment => {},
		OpReg::PeriodicListBase => {},
		OpReg::AsyncListAddr => {},
		OpReg::ConfigFlag => {},
		OpReg::PortSc0 => panic!("Attempted to directly write PortSc0"),
		}
		self.base.write_32(self.cap_length + reg as usize * 4, v)
	}

	/// Port Status and Control Register
	pub fn read_port_sc(&self, index: u8) -> u32 {
		assert!(index < 16);
		debug_assert!(index < (self.hcs_params() & 0xF) as u8);
		// SAFE: Reading is safe
		unsafe { self.base.read_32(self.cap_length + (OpReg::PortSc0 as usize + index as usize) * 4) }
	}
	/// (Write) Port Status and Control Register
	pub unsafe fn write_port_sc(&self, index: u8, v: u32) {
		assert!(index < 16);
		debug_assert!(index < (self.hcs_params() & 0xF) as u8);
		#[cfg(debug_assertions)]
		{
			// TODO: Validate written value
		}
		self.base.write_32(self.cap_length + (OpReg::PortSc0 as usize + index as usize) * 4, v)
	}
}



pub const USBCMD_Run            : u32 = 0x0001;
pub const USBCMD_HCReset        : u32 = 0x0002;
pub const USBCMD_PeriodicEnable : u32 = 0x0010;
pub const USBCMD_AsyncEnable    : u32 = 0x0020;
/// Interrupt on Async Advance Doorbell
/// 
/// Requests `USBINTR_IntrAsyncAdvance` for the next time the async queue advances
pub const USBCMD_IAAD           : u32 = 0x0040;

/// Interrupt on completion (also for USBSTS)
pub const USBINTR_IOC               : u32 = 0x0001;
/// A bus error has been detected
pub const USBINTR_Error             : u32 = 0x0002;
pub const USBINTR_PortChange        : u32 = 0x0004;
pub const USBINTR_FrameRollover     : u32 = 0x0008;
pub const USBINTR_HostSystemError   : u32 = 0x0010;
pub const USBINTR_IntrAsyncAdvance  : u32 = 0x0020;

/// The host controller is halted
pub const USBSTS_HcHalted        : u32 = 0x1000;
/// The async queue is empty
pub const USBSTS_Reclamation     : u32 = 0x2000;
/// 
pub const USBSTS_PeriodicEnabled : u32 = 0x4000;
/// 
pub const USBSTS_AsyncEnabled    : u32 = 0x4000;


pub const PORTSC_CurrentConnectStatus: u32 = 0x0001;
pub const PORTSC_ConnectStatusChange : u32 = 0x0002;
pub const PORTSC_PortEnabled         : u32 = 0x0004;
pub const PORTSC_PortEnableChange    : u32 = 0x0008;
pub const PORTSC_OvercurrentActive   : u32 = 0x0010;
pub const PORTSC_OvercurrentChange   : u32 = 0x0020;
pub const PORTSC_ForcePortResume     : u32 = 0x0040;
pub const PORTSC_Suspend             : u32 = 0x0080;
pub const PORTSC_PortReset           : u32 = 0x0100;
pub const PORTSC_Reserved1           : u32 = 0x0200;
pub const PORTSC_LineStatus_MASK     : u32 = 0x0C00;
pub const PORTSC_LineStatus_SE0      : u32 = 0x0000;
pub const PORTSC_LineStatus_Jstate   : u32 = 0x0400;
pub const PORTSC_LineStatus_Kstate   : u32 = 0x0800;
pub const PORTSC_LineStatus_Undef    : u32 = 0x0C00;
pub const PORTSC_PortPower           : u32 = 0x1000;
pub const PORTSC_PortOwner           : u32 = 0x2000;
pub const PORTSC_PortIndicator_MASK  : u32 = 0xC000;
pub const PORTSC_PortIndicator_Off   : u32 = 0x0000;
pub const PORTSC_PortIndicator_Amber : u32 = 0x4000;
pub const PORTSC_PortIndicator_Green : u32 = 0x8000;

