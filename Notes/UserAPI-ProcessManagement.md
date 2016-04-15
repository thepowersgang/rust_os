% Userland - Process Management

Child Process Startup
=====================

`CORE_STARTPROCESS` syscall
----
Only called by the loader, takes a name for the new process and a region of the current process to clone.

Returns an object handle representing the child process (not yet running)

This handle can be used for object handover and other pre-boot configuration.

To start execution, the `_START` call is used, which takes the initial IP and SP values, and returns the runtime handle to the process


Child Object Passing
====================
The parent can transfer an arbitary number of objects to the child process before it begins execution. These are popped by the child in a FIFO fashion.

The loader uses this to send the executable handle (and currently, the root handle as well).

