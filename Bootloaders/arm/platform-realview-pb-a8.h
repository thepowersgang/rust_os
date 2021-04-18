
#define RAM_START	0x00000000	// Realview RAM starts at 0
#define RAM_LENGTH	0x10000000	// 128MB (a safe assumption)
#define UART_BASE	0x10009000	// PL011 UART (first of 4)
//#define FDT_BASE	RAM_START	// No provided FDT?

#define fdt_start	_binary_fdt_realview_pb_a8_dtb_start
#define fdt_end		_binary_fdt_realview_pb_a8_dtb_end

