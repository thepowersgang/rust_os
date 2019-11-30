
Userland API Requirements/Desires
=================================

- Simple sync (or aparently synch) operations should exist
  - This requires that the kernel API cheaply accept arbitary source/destination buffers?

- Support zero-copy operations where possible
  - Optional for the first pass
  - Want to be able to have the code as efficient as possible?
- Support async writes (which with zero-copy requires deferred IO support)
  - Is this really needed? Wouldn't you just use a separate thread?
  - Could be async with a kernel-managed buffer (that could be mapped into user-space on request)

- Fixed (or controllable) memory overheads for operations.

- Integrate into the async/await language support
  - These return futures, so need somewhere to store the future (with controlled lifetimes)
  - A per-thread futures executor? Or per-process?
  - User-mode can use (effectively) signals to signal the executor?


Problems / Edge-cases
=====================

- Multiple semi-sequential operations
  - Reading filesystem metadata
  - Fragmented files (multiple low-level reads required for a contigious request)

- RAID/JBOD volumes
  - Multiple requests, but intended to be performed in parallel

- Nested operations
  - Virtual block devices (backed by filesystem, or network)

- Queuing support (have two input buffers ready, so there's no gap in processing that can cause stalls/loss)
  - I.e. have one buffer actively being transmitted/filled, with another waiting for the previous to be completed)

- Returning futures from dynamic dispatch (currently used for disk/network drivers)
  - Requires passing a buffer in which to store the returned future? Or returning a Boxed future
  - Maybe passing a call-local allocator? Or use a thread-local allocator?
    - An allocator that's cleared when the async operation completes could work.
    - Needs `free` support though, as some async operations may invoke other dynamic futures
    - Per-handle allows for tighter control, but may be inefficient.
    - The actual pool needs to be in global space? Maybe not, the futures always run in the same thread (or at least the same process)

