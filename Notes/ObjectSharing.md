% API and Principles of sharing syscall objects between processes

Requirements
===
- Shared objects named?
 - Methods: "well known" IDs?, Attached strings?
 - Why: VFS uses a set of them for general access
- Dynamic transmission (needs to go across a general IPC channel)
 - Why: The file and socket service needs to give handles out


Ideas
===

Side channel on IPC
---
- Single slot per IPC channel?

