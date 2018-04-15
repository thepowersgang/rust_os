
Userland API Requirements/Desires
=================================

- Simple sync (or aparently synch) operations should exist
  - This requires that the kernel API cheaply accept arbitary source/destination buffers?

- Support zero-copy operations where possible
  - Optional for the first pass
  - Want to be able to have he code as efficient as possible?
- Support async writes (which with zero-copy requires deferred IO support)
  - Is this really needed? Wouldn't you just use a separate thread?
  - Could be async with a kernel-managed buffer (that could be mapped into user-space on request)



