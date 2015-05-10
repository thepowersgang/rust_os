Physical memory book-keeping
===


Notes from a discussion on #osdev
---
* Reference count (n-bits), 32 will suffice, 16 could work too... but might be squeezy
* Backing store ID (32 bits, in Tifflin, this will be a block device... probably)
* Backing store offset (64-bits, page number in backing)
 * 64 for forward compat

Questions
---
* Should mapped pages be associated with a block device, inode/file?
 * BlockDev allows caching of everything, and easy mapping back to the store
 * Inode (well, a inode cache ID) is higher level, and makes unmound flush easier
* Would giving each PID its own inode for memory be a good idea?
 * This requires the inode model, but allows nice unified handling of mmap


VFS Itself
===

Node Types
---
* Normal File (with alternate streams)
 * File data cache.
* Directory
 * With cache?
* Symbolic Link
* UNIX Device (or other special)

```
trait NodeBase { ... }
trait File: NodeBase { ...  }
trait Dir: NodeBase { ... }
trait Symlink: NodeBase { ... }
trait Special: NodeBase { ... }

enum Node
{
	File(Box<File>),
	Dir(Box<Dir>),
	Symlink(Box<Symlink>),
	Special(Box<Special>),
}
static S_NODECACHE: Map<GlobalNodeId,Node>;
```

Filesystem Drivers
---
Driver itself provides a detect and mount method. Returns a mountpoint structure (special case of directory node)


Mountpoints
---

Prefix map of string (or pre-broken path?) to boxed filesystem roots

TODO: Would a sorted list work well for this? Or use another DS.
* Sorted list of `'static` decomposed paths? (Special type)
* Just use a string, and do a prefix match?

Symbolic Links
---
Naive model is to restart parsing when a symlink is encountered, absolutising the new path (CUR + LINK).
*Not sure if this is the correct model to be using, possible problems with expected POSIX model? That said, pick the most sensible model*

Access API
---

A single `open` method for all node types

* Takes argument on what type to expect (Any, File, Dir, Link, Other)
* Type changes the valid set of operations on the handle
* Various open modes with rules for files (SharedRO, ExclRW, UniqueRW, Append, Unsynch)

