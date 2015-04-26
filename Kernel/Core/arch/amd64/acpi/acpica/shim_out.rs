
use super::shim_ext::*;

#[no_mangle]
#[linkage="external"]
extern "C" fn AcpiOsInitialize() -> ACPI_STATUS
{
	AE_OK
}
