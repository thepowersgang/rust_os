#![allow(dead_code)]

/// Controller registers
pub struct Regs
{
	io: ::kernel::device_manager::IOBinding,
	caplength: u8,
	dboff: u32,
	rtsoff: u32,
}

impl Regs
{
	/// UNSAFE: Caller must ensure that the IO is XHCI
	pub unsafe fn new(io: ::kernel::device_manager::IOBinding) -> Regs {
		log_debug!("{io:?} version={version:#x}", version=io.read_16(2));
		let caplength = io.read_8(0);
		let dboff = io.read_32(0x14);
		let rtsoff = io.read_32(0x18);
		log_debug!("- caplength={caplength}, dboff={dboff:#x}, rtsoff={rtsoff:#x}");
		Regs {
			io,
			caplength,
			dboff,
			rtsoff,
		}
	}
}

/// Raw capability registers
impl Regs
{
	#[allow(dead_code)]
	pub fn hci_version(&self) -> u16 {
		// SAFE: Read-only register
		unsafe { self.io.read_16(2) }
	}
	fn hcs_params_1(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.io.read_32(4) }
	}
	fn hcs_params_2(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.io.read_32(8) }
	}
	fn hcs_params_3(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.io.read_32(12) }
	}

	/// "Capability Parameters 1"
	/// - 0: "AC64" 64-bit Addressing Capability
	/// - 1: "BWC" BW Negotiation Capability
	/// - 2: "CSZ" Context Size
	/// - 3: "PPC" Port Power Control
	/// - 4: "PIND" Port Indicators
	/// - 5: "LHRC" Light HC Reset Capability
	/// - 6: "LTC" Latency Tolerance Messaging Capability
	/// - 7: "NSS" No Secondary SID Support
	/// - 8: "PAE" Parse All Event Data
	/// - 9: "SPC" Stopped - Short Packet Capability
	/// - 10: "SEC" Stopped EDTLA Capability
	/// - 11: "CFC" Contiguous Frame ID Capability
	/// - 12-15: "MaxPSASize" Maximum Primary Stream Array Size
	/// - 16:31: "ECP" xHCI Extended Capabilities Pointer
	pub fn hcc_params_1(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.io.read_32(0x10) }
	}

	/// Offset of the doorbell registers (relative to base)
	/// NOTE: The value is 4 byte aligned
	#[cfg(any())]	// Already read
	pub fn dboff(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.io.read_32(0x14) }
	}
	/// Offset of the Runtime Registers (relative to base)
	/// NOTE: The value is 32-byte aligned
	#[cfg(any())]	// Already read
	pub fn rtsoff(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.io.read_32(0x18) }
	}

	/// "Capability Parameters 2"
	/// - 0: "U3C" U3 Entry Capability
	/// - 1: "CMC" Configure Endpoint Command Max Exit Latency Too Large Capability
	/// - 2: "FSC" Force Save Context Capability
	/// - 3: "CTC" Compliance Transition Capability
	/// - 4: "LEC" Large ESIT Payload Capability
	/// - 5: "CIC" Configuration Information Capability
	/// - 6: "ETC" Extended TBC Capability
	/// - 7: "ETC_TSC" Extended TBC TRB Status Capability
	/// - 8: "GSC" Get/Set Extended Property Capability
	/// - 9: "VTC" Virtualization Based Trusted I/O Capability
	pub fn hccparams2(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.io.read_32(0x1C) }
	}
}

/// HCSPARAMS1 - Structural parameters 1
/// 
/// Limits mostly.
impl Regs
{
	/// Number of devices supported by the controller
	pub fn max_slots(&self) -> u8 {
		((self.hcs_params_1() >> 0) & 0xFF) as u8
	}
	/// Number of "interrupters" (interrupt endpoints? MSI vectors)
	pub fn max_intrs(&self) -> u16 {
		((self.hcs_params_1() >> 8) & 0x3FF) as u16
	}
	/// Number of ports in the root hub (architectually up to 255)
	pub fn max_ports(&self) -> u8 {
		((self.hcs_params_1() >> 24) & 0xFF) as u8
	}
}

/// HCSPARAMS2 - Structural parameters 2
impl Regs
{
	/// Isochronous Scheduling Threshold
	/// 
	/// How much headway the controller needs to hit the isoch timing
	pub fn ist(&self) -> u8 {
		((self.hcs_params_2() >> 0) & 0xF) as u8
	}
	pub fn erst_max(&self) -> u8 {
		((self.hcs_params_2() >> 4) & 0xF) as u8
	}
	/// High 5 bits of the number of scratchpad buffers required by the controller
	pub fn max_scratchpad_buffers_hi(&self) -> u16 {
		((self.hcs_params_2() >> 21) & 0x1F) as u16
	}
	/// Indicates if the controller requires the driver to keep the scratchpad valid across suspend/restore
	pub fn scratchpad_restore(&self) -> bool {
		self.hcs_params_2() & (1 << 26) != 0
	}
	/// Low 5 bits of the number of scratchpad buffers required by the controller
	pub fn max_scratchpad_buffers_lo(&self) -> u16 {
		((self.hcs_params_2() >> 27) & 0x1F) as u16
	}

	/// The number of scratchpad buffers required by the controller
	/// - Max of 1024 (5+5 bits)
	pub fn max_scratchpad_buffers(&self) -> u16 {
		self.max_scratchpad_buffers_hi() << 5 | self.max_scratchpad_buffers_lo()
	}
}

/// HCSPARAMS3 - Structural parameters 3
impl Regs
{
	pub fn u1_device_exit_latency(&self) -> u16 {
		((self.hcs_params_3() >> 0) & 0xFFFF) as u16
	}
	pub fn u2_device_exit_latency(&self) -> u16 {
		((self.hcs_params_3() >> 16) & 0xFFFF) as u16
	}
}

/// Run/stop
pub const USBCMD_RS: u32 = 1 << 0;
/// Host Controller Reset
pub const USBCMD_HCRST: u32 = 1 << 1;
/// Interrupter Enable
pub const USBCMD_INTE: u32 = 1 << 2;

/// Host Controller Halted
pub const USBSTS_HCH : u32 = 1 << 0;
//
pub const USBSTS_HSE : u32 = 1 << 2;
/// Event Interrupt
pub const USBSTS_EINT: u32 = 1 << 3;
/// Port Change Detect
pub const USBSTS_PCD : u32 = 1 << 4;
/// Controller not ready
pub const USBSTS_CNR : u32 = 1 << 11;
/// Host Controller Error
pub const USBSTS_HCE : u32 = 1 << 12;

// CRCR: Ring Cycle State
pub const CRCR_RCS: u64 = 1 << 0;

macro_rules! def_op_regs {
	(
		$( $(#[$doc:meta])* reg $name:ident: $ty:ident[$mask:expr] = $ofs:literal; )*
	) => {
		$(
			$(#[$doc])*
			pub struct $name<'a>(&'a Regs);
			impl $name<'_> {
				fn ofs(&self) -> usize {
					self.0.op_ofs($ofs)
				}
				pub fn read(&self) -> $ty {
					// SAFE: Reading XHCI registers is always safe
					unsafe { def_op_regs!(@do_read self -> $ty) }
				}
				pub unsafe fn write(&self, v: $ty) {
					assert!(v & !$mask == 0, "Reserved bits set in {}: {:#x} & {:#x} != 0", stringify!($name), v, $mask);
					def_op_regs!(@do_write v: $ty -> self)
				}
			}
		)*
	};
	(@do_read $self:ident -> u32) => { $self.0.io.read_32($self.ofs()) };
	(@do_write $v:ident : u32 -> $self:ident) => { $self.0.io.write_32($self.ofs(), $v) };
	(@do_read $self:ident -> u64) => { $self.0.io.read_64($self.ofs()) };
	(@do_write $v:ident : u64 -> $self:ident) => { $self.0.io.write_64($self.ofs(), $v) };
}
def_op_regs!{
	reg UsbCmd: u32[!0] = 0;
	reg UsbSts: u32[!0] = 4;
	reg DNCtrl: u32[0xFFFF] = 0x14;
	reg Crcr: u64[!0] = 0x18;
	reg DCBAAP: u64[!0x1F] = 0x30;
	reg Config: u32[0x3FF] = 0x38;
}

/// Operational registers
impl Regs
{
	pub fn op_ofs(&self, ofs: usize) -> usize {
		self.caplength as usize + ofs
	}

	/// USB Command Register
	/// - 0: Run/Stop [USBCMD_RS]
	/// - 1: Host Controller Reset [USBCMD_HCRST]
	pub fn usbcmd(&self) -> UsbCmd<'_> {
		UsbCmd(self)
	}

	/// USB Status Register
	/// - 0: Host Controller Halted [USBSTS_HCH]
	/// - 2: ? [USBSTS_HSE]
	/// - 3: Event Interrupt [USBSTS_EINT]
	/// - 4: Port Change Detect [USBSTS_PCD]
	/// - 11: Controller not ready [USBSTS_CNR]
	/// - 12: Host Controller Error [USBSTS_HCE]
	pub fn usbsts(&self) -> UsbSts<'_> {
		UsbSts(self)
	}

	/// A bit-mask of supported page sizes, with bit 0 being 0x1000, 1 being 0x2000
	pub fn pagesizes(&self) -> u32 {
		// SAFE: Read is safe
		unsafe { self.io.read_32(self.op_ofs(8)) }
	}

	/// DNCTRL - Device Notification ConTRoL register
	/// 
	/// This is a bitmask for the 16 notification events
	pub fn dnctrl(&self) -> DNCtrl<'_> {
		DNCtrl(self)
	}

	/// CRCR - Command Ring Control Register
	/// 
	/// 0: Ring Cycle State (RCS), writes are ignored unless `CCR` reads as `0`
	/// 1: Command Stop (CS) - Write a 1 to cleanly stop processing
	/// 2: Command Abort (CA) - Write a 1 to abort processing of the command queue
	/// 3: Command Ring Running (CCR) - Read-Only state of the command ring processing
	/// 6:64: Command Ring Pointer, writes are ignored unless `CCR` reads as `0`
	pub fn crcr(&self) -> Crcr<'_> {
		Crcr(self)
	}

	/// DCBAAP - Device Context Array Pointer
	pub fn dcbaap(&self) -> DCBAAP<'_> {
		DCBAAP(self)
	}

	/// CONFIG - Configure register
	/// 
	/// 7:0: Max Device Slots Enabled
	/// 8  U3 Entry Enable (U3E)
	/// 9: Configuration Information Enable (CIE)
	/// 
	/// NOTE: Writes to this impact the size of the structure pointed to by [Self::dcbaap]
	pub fn config(&self) -> Config<'_> {
		Config(self)
	}

	/// Accessor for per-port registers
	pub fn port(&self, index: u8) -> PortRegs<'_> {
		PortRegs { parent: self, index }
	}
}

pub struct PortRegs<'a>
{
	parent: &'a Regs,
	index: u8,
}
impl PortRegs<'_>
{
	fn ofs(&self, ofs: usize) -> usize {
		self.parent.caplength as usize + 0x400 + 0x10 * self.index as usize + ofs
	}
	/// PORTSC: Status and control
	/// 
	/// - 0: Current Connect Status (CCS)
	/// - 1: Port Enabled/Disabled (PED)
	pub fn sc(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.parent.io.read_32(self.ofs(0)) }
	}
	pub fn set_sc(&self, v: u32) {
		// SAFE: No memory unsafety to PORTSC
		unsafe { self.parent.io.write_32(self.ofs(0), v) }
	}
	/// Power management status and control
	pub fn pmsc(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.parent.io.read_32(self.ofs(4)) }
	}
	/// Link Info
	pub fn li(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.parent.io.read_32(self.ofs(8)) }
	}
	/// Hardware LPM Control
	pub fn hlpmc(&self) -> u32 {
		// SAFE: Read-only register
		unsafe { self.parent.io.read_32(self.ofs(12)) }
	}
}

pub const PORTSC_CCS: u32 = 1 << 0;
pub const PORTSC_PED: u32 = 1 << 1;

impl Regs
{
	pub fn ring_doorbell(&self, index: u8, value: u32) {
		// SAFE: Doorbells are just notifications
		unsafe {
			self.io.write_32(self.dboff as usize + index as usize * 4, value)
		}
	}
}

/// Runtime registers
impl Regs
{
	/// Current microframe index
	pub fn mfindex(&self) -> u32 {
		// SAFE: Reading is safe
		unsafe { self.io.read_32(self.rtsoff as usize + 0x00) }
	}

	/// Interrupters (up to 1024 of them)
	/// 
	/// - When using the legacy IRQ, only `0` is valid
	/// - When using MSI, only 0-15 are valid
	/// - And with MSI-X, all 1024 are valid
	pub fn interrupter(&self, index: u16) -> Interrupter<'_> {
		assert!(index < 1024);
		Interrupter { parent: self, index }
	}
}


macro_rules! def_int_regs {
	(
		$( $(#[$doc:meta])* reg $name:ident: $ty:ident[$mask:expr] = $ofs:literal; )*
	) => {
		$(
			$(#[$doc])*
			pub struct $name<'a>(&'a Interrupter<'a>);
			impl $name<'_> {
				fn ofs(&self) -> usize {
					self.0.ofs($ofs)
				}
				pub fn read(&self) -> $ty {
					// SAFE: Reading XHCI registers is always safe
					unsafe { def_int_regs!(@do_read self -> $ty) }
				}
				pub unsafe fn write(&self, v: $ty) {
					assert!(v & !$mask == 0, "Reserved bits set in {}: {:#x} & {:#x} != 0", stringify!($name), v, $mask);
					def_int_regs!(@do_write v: $ty -> self)
				}
			}
		)*
	};
	(@do_read $self:ident -> u32) => { $self.0.parent.io.read_32($self.ofs()) };
	(@do_write $v:ident : u32 -> $self:ident) => { $self.0.parent.io.write_32($self.ofs(), $v) };
	(@do_read $self:ident -> u64) => { $self.0.parent.io.read_64($self.ofs()) };
	(@do_write $v:ident : u64 -> $self:ident) => {{
		$self.0.parent.io.write_32($self.ofs(), $v as u32);
		$self.0.parent.io.write_32($self.ofs()+4, ($v >> 32) as u32);
	}};
}
def_int_regs!{
	reg IMan: u32[!0] = 0;	// TODO: Safe to write
	reg ERSTSz: u32[0xFFFF] = 8;
	reg ERSTBA: u64[!0] = 0x10;
	reg ERDP: u64[!0] = 0x18;
}
pub struct Interrupter<'a>
{
	parent: &'a Regs,
	index: u16,
}

/// ERDP: Event Handler Busy (EHB) - Indicates that there is an event raised
pub const ERDP_EHB: u64 = 1 << 3;

impl Interrupter<'_>
{
	fn ofs(&self, ofs: usize) -> usize {
		// The first 0x20 bytes just has the MFINDEX in it
		self.parent.rtsoff as usize + 0x20 + 0x20 * self.index as usize + ofs
	}
	/// IMAN - Interrupter management
	/// 
	/// - 0: IP - Interrupt pending (RW1C)
	/// - 1: IE - Interrupt enable (RW)
	pub fn iman(&self) -> IMan<'_> {
		IMan(self)
	}
	/// IMOD - Interrupter Moderation
	/// 
	/// - 15:0 : IMODI (Interrupt Moderation Interval)
	/// - 31:16: IMODC (Interrupt Moderation Counter)
	pub fn imod(&self) -> u32 {
		// SAFE: Reading has no effect
		unsafe { self.parent.io.read_32(self.ofs(4) ) }
	}
	/// ERSTSZ - Event Ring Segment Table Size
	/// 
	/// - 15:0 - Number of entries
	pub fn erstsz(&self) -> ERSTSz<'_> {
		ERSTSz(self)
	}
	/// ERSTBA - Event Ring Segment Table Base Address
	/// 
	/// Points to an array of Address,Size pairs that specifies the event ring.
	/// Length is specified in [Self::erstsz]
	pub fn erstba(&self) -> ERSTBA<'_> {
		ERSTBA(self)
	}

	/// ERDP - Event Ring Dequeue Pointer
	/// 
	/// - 2:0: Dequeue ERST Segment Index (DESI) - RW
	///   - Low three bits of the `ERSTBA` entry this pointer is within.
	/// - 3: Event Handler Busy (EHB) - RW1C [ERDP_EHB]
	/// - 63:4: Event Ring Dequeue Pointer
	pub fn erdp(&self) -> ERDP<'_> {
		ERDP(self)
	}

}
