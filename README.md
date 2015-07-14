"Tifflin" Experimental Kernel (and eventually Operating System)
=====

This is an experiment in writing an OS Kernel in rust (http://rust-lang.org).

Mostly the architecture is being designed as I go along, but it will be written to be architecture independent (the current verison is x86\_64/amd64).

## Design Features
- Runtime module initialisation with dependencies
- Clear user-kernel separation of duties
 - Userland owns the ELF loader, kernel uses a custom format for init.
- Object-based syscall API



## Build Dependencies
`nasm` `imagemagick`
