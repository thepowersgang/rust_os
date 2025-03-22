"Tifflin" Experimental Kernel (and eventually Operating System)
=====

This is an experiment in writing an OS Kernel in rust (http://rust-lang.org).

Mostly the architecture is being designed as I go along, but it will be written to be architecture independent (the current version is x86\_64/amd64).

# Testing
With the required dependencies installed, run `make -C rundir ENABLE_VIDEO=1` to do a standard qemu PXE boot with display output enabled.

## Design Features
- Runtime module initialisation with dependencies
- Clear user-kernel separation of duties
 - Userland owns the ELF loader, kernel uses a custom format for init.
- Object-based syscall API
- Kernel-provided window manager (yes, I know old windows did this)

## Progress
- Filesystems
 - ISO9660
 - FAT12/16/32
- Storage
 - (P)ATA
 - SATA (AHCI)
 - ATAPI CDROM
 - VirtIO Block
- Input
 - PS2 Keyboard/Mouse
- Graphics
 - Multiboot only
- GUI Apps
 - Login (Credentials are root/password)
 - "GUI Shell" (with background!)
 - Text Terminal app (with basic set of commands)
 - Filesystem viewer
- Architectures
 - amd64 (aka x86\_64) - Boots to limit of implementation
 - armv7 - Loads userland then crashes


# Dependencies
## Build Dependencies
- `nasm`
- GNU Binutils (cross-compiled)
- GCC (for ACPICA)

## Running
- `qemu-system-amd64` (or other `qemu-system-*` binary)
- `mtools` (for making FAT volumes)
- PXE Boot: `pxelinux` (for doing PXE boots, the default)
- ISO Boot: `grub-mkrescue` (for ISO boots)
- EFI Boot: `OVMF` qemu firmware
- `libguestfs-tools` (for creating generic disk images)
