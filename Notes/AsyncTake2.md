
Asynchronous IO (and other waiting) - Attempt 2


Requirements:
===

- Low to no kernel memory overhead
- Prefer no need for custom userland allocator
- Obeys rust's memory safety rules
- Read+Write
- No arbitary callbacks (see memory safety)
- Allows state continuations in kernel (e.g. VFS stalling on disk reads)


Existing Models
===

- Windows IOCP:
 - An object that recieves messages whenever an "overlapped IO" operation completes
 - RIO Extensions allow setting up re-waits automatically
- POSIX select
 - Call that wakes when a handle is ready for IO
- Linux epoll
 - Same as above, but different API


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
	/// Userland's handle ID for the relevant object
	handle: u32,
	/// A per-
	state: u32,

	position: u64,

	is_read: bool,
	data: *mut [u8],
}
```

Kernel API
===
- Registered async providers (e.g. TCP, ATA, SATA, ...)
- Async queue at syscall layer
 - Contains metadata "blob" from when the async op was registered with the provider
 - Blob could be a StackDST, or even just an opaque piece of data with a signature...
  - (That's a StackDST really)

```rust
struct KAsyncDesc
{
	provider: AsyncProviderHandle,
	provider_handle: usize,
	// FreezeRaw - Like Freeze, but doesn't expose safe access
	data: FreezeRaw<[u8]>,
}
```

User API
===

```rust
/// Starts an async read/write for this handle
/// Will block until the low-level operation is queued (and the buffer is "locked").
/// Returns a handle to the async operation.
fn async_start(handle: u32, offset: u64, is_read: bool, buffer: *mut [u8]) -> u32;
/// Wait for any of the given async handles to complete
fn async_wait(async_handles: &[u32], statuses: &mut [bool]) -> u32;
/// Cancel an in-progress async operation
fn async_cancel(async_handle: u32);
/// Release an async handle back into the free pool, returning the number of bytes processed in its lifetime
fn async_release(async_handle: u32) -> Result<usize>;
```

