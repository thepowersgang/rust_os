

#[repr(C)]
#[derive(PartialEq,Eq)]
pub struct ACPI_STATUS(i32);

const AE_CODE_ENVIRONMENTAL: i32 = 0x0000;
const AE_CODE_PROGRAMMER   : i32 = 0x1000;

pub const AE_OK             : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 0);
pub const AE_ERROR          : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 1);
pub const AE_NO_ACPI_TABLES : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 2);
pub const AE_NO_MEMORY      : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 4);
pub const AE_NOT_FOUND      : ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL| 5);
pub const AE_NOT_IMPLEMENTED: ACPI_STATUS = ACPI_STATUS(AE_CODE_ENVIRONMENTAL|14);
pub const AE_BAD_PARAMETER: ACPI_STATUS = ACPI_STATUS(AE_CODE_PROGRAMMER|1);

impl ::core::fmt::Display for ACPI_STATUS {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match self.0
		{
		0 => write!(f, "Success"),
		1 => write!(f, "Misc Error"),
		_ => write!(f, "Unknown({})", self.0),
		}
	}
}