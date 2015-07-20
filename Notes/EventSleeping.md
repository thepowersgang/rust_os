% Tifflin Design Notes: Event Sleeping (userland)

Need an API to allow userland threads to sleep until an event occurs. Since all threads are relatively heavy, this should be a general-purpose system, where any type of event can cause a wakeup.

# Existing Options
- `select`/`epoll`
 - `epoll`'s API uses a list of objects on which to wait
 - Pretty memory efficent
  - User provides list of object,mask, which can be turned into similar in the kernel
- Callbacks
 - Interesting thread-wise, maybe if there's a function that is basically "wait for callback" that a thread has to run?
 - Could have a registered callback select if it can be multi-threaded or not
 - Can be used to implement other forms
 - Downside: Heavy on allocations (depends?)
  - Kernel needs to remember user's provided state, and what registered callbacks there are.
- Blocking only
 - Simplest of the lot, but far harder to audit/debug (and no async support)

# Chosen Option
An event list model (listing objects and wait flags) seems the easiest one (and matches the existing async code)

- Place an extra method on syscalls::Object trait that takes an event mask and a wait object, and registers waits


<!--- vim: set ft=markdown: -->
