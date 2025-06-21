#ifndef __ACRUST_H__
#define __ACRUST_H__

/* Building for rust bindings uses GCC */

#include <stdint.h>
#include <stdarg.h>

// TODO: Handle this being different
#define ACPI_MACHINE_WIDTH	64

#define COMPILER_DEPENDENT_INT64        int64_t
#define COMPILER_DEPENDENT_UINT64       uint64_t

#define ACPI_UINTPTR_T      uintptr_t

#define ACPI_USE_DO_WHILE_0
#define ACPI_USE_LOCAL_CACHE

#undef ACPI_USE_SYSTEM_CLIBRARY
#undef ACPI_USE_STANDARD_HEADERS
//#undef ACPI_USE_NATIVE_DIVIDE

#include "acgcc.h"

// Make AcpiOsPrintf call AcpiOsVprintf, which rust can then handle
extern void AcpiOsVprintf(const char *fmt, va_list args);
static inline void AcpiOsPrintf(const char *fmt, ...) {
	va_list	args;
	va_start(args, fmt);
	AcpiOsVprintf(fmt, args);
	va_end(args);
}

#endif

