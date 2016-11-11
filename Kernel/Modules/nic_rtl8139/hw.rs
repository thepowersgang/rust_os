
#[repr(u16)]
#[allow(dead_code,non_camel_case_types)]
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

