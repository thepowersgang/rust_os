
fn main()
{
	let read_handle = AsyncWrapper::new( vfs::FileHandle::open("/foo.txt") );
	let write_handle = AsyncWrapper::new( vfs::FileHandle::open("/bar.txt") );
	let mut rdbuf = [0; 512];

	read_handle.async_read( (0), &mut rdbuf);
	write_handle.async_write( (0), b"Hello, World\n");

	loop
	{
		match async::wait_one( &[read_handle.inner(), write_handle.inner()] )
		{
		None => break,
		Some(0) => println!("Read complete"),
		Some(1) => println!("Write complete"),
		Some(_) => unreachable!(),
		}
	}
}

mod async
{
	pub struct AsyncObject
	{
		waiter: AtomicPtr<SleepObject>,
		result: Spinlock<Option<usize>>,
		stack: Spinlock<::stack_dst::Stack<Layer>>,
	}

	struct SleepObjectReg<'a>
	{
		so: &'a SleepObject,
		handles: &'a [&'a AsyncObject],
	}
	impl<'a> SleepObjectReg<'a>
	{
		fn new(so: &SleepObject, handles: &[&AsyncObject]) -> Result<SleepObjectReg, ()>
		{
			let rv = SleepObjectReg {
				so: so,
				handles: handles,
				};
			for h in handles
			{
				if h.waiter.compare_and_swap(::core::ptr::null_mut(), so as *const _ as *mut _, Ordering::SeqCst) != ::core::ptr::null_mut()
				{
					// Uh-oh, something else is waiting on this?
					return Err( () );
				}
			}
			Ok(rv)
		}
	}
	impl<'a> ops::Drop for SleepObjectReg<'a>
	{
		fn drop(&mut self)
		{
			for h in self.handles
			{
				h.waiter.compare_and_swap(self.so as *const _ as *mut _, ::core::ptr::null_mut(), Ordering::SeqCst);
			}
		}
	}

	struct AsyncStackPush<'a>
	{
		_pd: PhantomData<&'a mut AsyncStack>,
		ptr: *mut AsyncStack,
	}
	impl<'a> AsyncStackPush<'a>
	{
		fn new_with_top(stack: &mut AsyncStack) -> (AsyncStackPush, &mut Layer)
		{
			(AsyncStackPush { _pd: PhantomData, stack }, stack.top_mut(), )
		}
		fn push(&mut self, v: impl Layer)
		{
			unsafe {
				(*self.ptr).push(v);
			}
		}
	}

	pub fn wait_one(handles: &[&AsyncObject]) -> Option< (usize, usize) >
	{
		// 0. Check if there are any active waiters in the list
		// - If not, return None
		if ! handles.iter().any(|h| !h.stack.lock().empty())
		{
			// - All of the handles had empty stacks
			return None;
		}

		let so = SleepObject::new("async::wait_one");

		// 1. Register the sleep object on all handles
		let _reg_handle = match SleepObjectReg::new(&so, handles)
			{
			Ok(v) => v,
			Err(_) => panic!("Waiting on a handle that already has a waiter"),
			};

		// 2. Loop until one of the items complete.
		loop
		{
			for (idx,h) in Iterator::enumerate(handles.iter())
			{
				let mut res = h.result.lock().take();
				while let Some(res) = res
				{
					// Result waiting!
					let mut stack_lh = h.stack.lock();
					// - Magic handle that only allows pushing to this (append-only) stack
					let (magic_handle, top) = AsyncStackPush::new_with_top(&mut stack_lh);

					res = top.advance(magic_handle, res);
					// If this returns non-None, then pop and continue
					if let Some(res) = res
					{
						stack_lh.pop();
						// If the last item was popped, return.
						if stack_lh.empty()
						{
							return Some( (idx, res) );
						}
					}
				}
			}

			// The above loop will early return if any of the items completed due to waiting state updates.
			// - Sleep until woken by an async update
			so.wait();
		}
	}
}


