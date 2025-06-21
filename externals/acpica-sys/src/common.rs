

pub type ACPI_SIZE = usize;
pub type ACPI_PHYSICAL_ADDRESS = u64;
pub type ACPI_IO_ADDRESS = u16;

pub type ACPI_PCI_ID = u32;

pub type ACPI_CPU_FLAGS = u32;
//pub type ACPI_SPINLOCK = *const crate::sync::Spinlock<()>;
//pub type ACPI_MUTEX = *const crate::sync::Mutex<()>;
//pub type ACPI_SEMAPHORE = *const crate::sync::Semaphore;

pub type ACPI_STRING = *mut u8;

pub type ACPI_THREAD_ID = u64;

#[repr(transparent)]
pub struct ACPI_HANDLE(*mut crate::Void);

#[repr(C)]
pub struct ACPI_BUFFER
{
	Length: u32, 
	Pointer: *const crate::Void,
}