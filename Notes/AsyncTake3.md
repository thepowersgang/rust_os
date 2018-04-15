
Requirements
============

- MUST support async waits on network sockets
- Should support completion model (buffer registration)
- Niceity: Queued buffer support (for heavy network operation)
- Niceity: Zero dynamic allocation in kernel
  - Would allow tracable kernel allocations.
  - Difficult.


Edge Cases
=========

Chained Operations
-----------------
E.g.
- Fragmented file read
  - Just keep looping at an async level, easy.
- Meatadata lookup (acceptable for that to be done in a worker or to be blocking)

Nested Operations
-----------------
- Virtual block devices
  - Worker thread?
  - Dynamic allocation if required?

Parrelel Operations
-------------------
- RAID/JBOD
  - Serialize?
  - Worker thread per RAID volume?

Rough design/example
======

Idea for a async API based around waiter objects and borrowed buffers.

TODO: Figure out how to handle layered APIs (e.g. FS that ends up doing multiple disk calls)
- Or when a read of a single file triggers multiple reads (e.g. from RAID, or from a JBOD array)

```rust
impl ReadBuffer<'a>
{
	// UNSAFE: If this is leaked while borrowed, the borrow will access invalidated memory
	unsafe fn new_borrow(data: &mut [u8]) -> ReadBuffer;
	fn new_user(data: FreezeMut<[u8]>) -> ReadBuffer<'static>;
	fn new_owned(size: usize) -> ReadBuffer<'static>;
	fn borrow(&self) -> ReadBufferHandle;
}
struct Waiter
{
	slots: Vec<AsyncStack>, 
}
impl Waiter
{
	fn new(max_events: usize) -> Waiter;
	fn get_handle(&self, index: usize) -> WaiterHandle;
	fn wait_event(&self) -> Option<(usize, usize)>;
}
impl Foo
{
	// This might hand these handles off to another thread (which would poke the waiter when data is read)
	fn do_async_op(&self, waiter: Handle, buffer: ReadBufferHandle);
}
```

When ReadBuffer is dropped, the borrow flag is checked - if set, panic (otherwise, continue)


```rust
let buf = [0; 32];
let waiter = Waiter::new(1);
// SAFE: Not leaked while borrowed
let readbuf = unsafe { ReadBuffer::new_borrow(&mut buf) };
foo.do_async_op(waiter.get_handle(1), readbuf.borrow());
while let Some(ev) = waiter.wait_event()
{
	match ev
	{
	(0, _) => {},
	(1, size) => {
		assert!( !readbuf.is_borrowed() );
		let data = &readbuf[..size]
		},
	_ = unreachable!(),
	}
}
```



Use a stack of async operations (using `stack_dst`'s stack support), which handles recursive calls to a limit (with the stack attached to the userland handle)
- When a stack is exhausted, it's extended on the heap (attached to the calling process?)

```rust
trait AsyncLayer
{
}

type AsyncStack<'a> = ::stack_dst::StackA<AsyncLayer+'a, [usize; 127]>;
```


Problems 
========

Follow-on operations
-------------------

E.g. Filesystem metadata lookup, non-contigious/cross-device reads

Some file read operations (or even network operations) will result in multiple low-level operations (e.g. reading a 1MB block of a file from
a filesystem with a 4KB block size, where the blocks are not contigious).

The lazy way is to serialise the multiple operations, keeping a stack of of async scopes that will need to repeat.
- This is the use of the `AsyncStack` type above.
- There will be one stack per async "operation" in the waiter (or just per async handle?), containing all of the data pertaining to that operation.


```
fn start_read_fdd(&self, waiter: WaiterHandle, sector: usize, dst: BufferHandle)
{
	struct State {
		lh: Option<LockHandle<'a>>,
	}
	waiter.push_state(move |waiter, val| {
		let lh = self.mutex.get_async_lock(val);
		lh.start_dma_rd(waiter, sector, dst);
		});
	self.mutex.async_lock(waiter);
}
```

