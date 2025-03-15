Kernel filesystem driver test framework

Uses a mixture of pre-built disk images and runtime-generated images

Dependencies:
- `guestfish` for copying files into volumes
- `sfdisk`
- `mkfs.ext2`
- `mkfs.vfat`
- `mkfs.ntfs`

NOTE: `guestfish` requires read access to the linux kernel image