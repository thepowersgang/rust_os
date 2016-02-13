
Asynchronous IO (and other waiting) - Attempt 2


Requirements:
===

- Low to no kernel memory overhead
- Prefer no need for custom userland allocator
- Obeys rust's memory safety rules
- Read+Write
- No arbitary callbacks (see memory safety)


Existing Models
===

- Windows IOCP:
 - An object that recieves messages whenever an "overlapped IO" operation completes
 - RIO Extensions allow setting up re-waits automatically
- POSIX select
 - Call that wakes when a handle is ready for IO


Notes:
===

- Has to be able to operate in parralel with a running user thread
 - Otherwise, if one async op completes and wakes the user, all others can't write.
 - Can state changes happen pure async? (E.g. when a lock is acquired)


Ideas
===

User-accessible async descriptors
---

- Per-process list?
- The user can request an async descriptor from the kernel (registering a buffer with it)
- Descriptor registration prepares the async op and optionally starts it
- Descriptor contains state information for the kernel (somehow)

- Kernel-side, a per-thread structure (in the TCB) is the async waiter


```rust
struct AsyncDesc
{
	handle: u32,
	state: u32,

	position: u64,

	is_read: bool,
	data: *mut [u8],
}
```

