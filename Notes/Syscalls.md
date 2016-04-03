% Tiffin Design Notes: Syscalls

# Rough Notes
- Well segmented (group syscalls by allocating n syscalls per subsystem)
- Generic API allowing easy definition of syscalls on both sides of the divide
- Object based?
 - Allow n methods on each object type (where objects are registered and managed by the syscall layer)
 - Possibly also allow attribute methods, not sure.

# Syscall Groups/Classes/Subsystems

## Processes and Threads
1. Log message
1. Spawn process
1. Exit process
1. Spawn thread
1. Terminate thread
1. Yield permissions
1. Lend permissions


## IPC
1. Create Named Server
1. Create Anon Server
1. Open Named Socket

### IPC Socket "Class"
 1. Send Blob
    Send a "datagram" packet to the other end of the socket.
 1. Send Object
 1. Recv Blob
 1. Recv Object

## Window Manager
1. New Group
1. New Window
 1. Wait for event
 1. Set title
 1. Resize
 1. Memory map
 1. Blit data
 1. Fill rect
 1. Trigger Redraw

## VFS
1. Open Any
 1. Get Type
1. Open File

<!--- vim: set ft=markdown: -->
