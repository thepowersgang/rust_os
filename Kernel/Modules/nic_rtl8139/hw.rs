// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/nic_rtl8139/hw.rs
//! Hardware definitions (registers and flags)
#![allow(dead_code)]

#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum Regs
{
	// MAC Address
	MAC0, MAC1, MAC2,
	MAC3, MAC4, MAC5,
	
	// Multicast Registers
	MAR0 = 0x08, MAR1, MAR2, MAR3,
	MAR4, MAR5, MAR6, MAR7,
	
	// Transmit status of descriptors 0 - 3
	TSD0 = 0x10,    TSD1 = 0x14,
	TSD2 = 0x18,    TSD3 = 0x1C,
	// Transmit start addresses
	TSAD0 = 0x20,   TSAD1 = 0x24,
	TSAD2 = 0x28,   TSAD3 = 0x2C,
	
	RBSTART = 0x30, // Recieve Buffer Start (DWord)
	// Early Recieve Byte Count
	ERBCR = 0x34,   // 16-bits
	// Early RX Status Register
	ERSR = 0x36,
	
	// -, -, -, RST, RE, TE, -, BUFE
	CMD     = 0x37,
	
	CAPR    = 0x38, // Current address of packet read
	CBA     = 0x3A, // Current Buffer Address - Total byte count in RX buffer
	
	IMR     = 0x3C, // Interrupt mask register
	ISR     = 0x3E, // Interrupt status register
	
	TCR     = 0x40, // Transmit Configuration Register
	RCR     = 0x44, // Recieve Configuration Register
	TCTR    = 0x48, // 32-bit timer (count)
	MPC     = 0x4C, // Missed packet count (due to RX overflow)
	
	CR_9346 = 0x50,
	CONFIG0 = 0x51,
	CONFIG1 = 0x52,
	// 0x53 resvd
	TIMERINT = 0x54,        // Fires a timeout when TCTR equals this value
}

pub const FLAG_ISR_SERR  : u16 = 0x8000;	// System error
pub const FLAG_ISR_TIMEO : u16 = 0x4000;	// Timer timeout (See TIMERINT)
pub const FLAG_ISR_LENCHG: u16 = 0x2000;	// Cable length changed
pub const FLAG_ISR_FOVW  : u16 = 0x0040;	// Rx FIFO Underflow
pub const FLAG_ISR_PUN   : u16 = 0x0020;	// Packet Underrung
pub const FLAG_ISR_RXOVW : u16 = 0x0010;	// Rx Buffer Overflow
pub const FLAG_ISR_TER   : u16 = 0x0008;	// Tx Error
pub const FLAG_ISR_TOK   : u16 = 0x0004;	// Tx OK
pub const FLAG_ISR_RER   : u16 = 0x0002;	// Rx Error
pub const FLAG_ISR_ROK   : u16 = 0x0001;	// Rx OK


pub const FLAG_TSD_TOK: u32 = 0x8000;


