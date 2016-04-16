Multiple-Root VFS Model
====

Executables get a set of directory handles, under well-known tag names
- (RO) CommonData
 - Folder containing the system's common resources (images etc)
- (RO) AppBin
 - Per-application binary directory (can be shared between binaries)
 - `/Applications/<appname>/bin`
- (RO) AppData
 - Per-application resource data directory
 - `/Applications/<appname>/data`
- (RW) AppStorage
 - Per-user-application read-write directory
 - `~/.AppData/<appname>/


Along with this, an application can expect other named handles depending in its use
- (RW) File
 - Read-write handle to a file handed for mutation. E.g. document to open
- (RO) Input
 - Read-only handle to a file/directory for input
 - e.g. A rustc-like program might get the directory of the source file, and the filename passed as a string parameter 
 - TODO: `rustc` would need to be able to access above the main.rs file in some cases (see this project's `syscalls.inc.rs` file)
- (RW) Output

Applications with an "application handle" (a kernel-registered non-forgable token) can make a request from a service to open a file/folder (either named or arbitary)
- This service can prompt the user, or accept the request (if whitelisted)


Application Handles
==================

Requirements
------------
- Not forgeable (presenting the handle via the kernel means that the communicating process was started by that program)
- Immutable
- Unique for each process
- Tied to a unique or unambigious user-readable name

Ideas
-----
- Kernel generates token based on Handle #1 (nominally the executable)
 - Downside: A malicious program could get a handle to a system app, then use a custom loader that loads handle #2
- Checksum (or just handle) of the first loaded executable section
 - Downside: Same as above
- Hash generated from all mapped executable files
 - Upside: Prevents malicious code from starting itself then stealing a program's key.
 - Downside: Vulnerable to DEP failures
 - Downside: Non-trivial mapping of hash/signature to application names
  - Fix: Can combine with loaded application manifiest that lists the app name and executables.
- Tie to the GUI window handle
 - Upside: Obvious origin (the window can be disabled), title shown taken from the window title.
 - Downside: Requires a complete rework of GUI handle distribution

Accepted Method
---------------

Use the application's window handle (which must be visible) as the modal parent of the file-open dialog. This is made possible
through special access granted to the session leader (holder of the window group handle, all other applications just have a root window).

Usecases
====

Application / Executable Loading
--------------------------------

A process should be able to start any application in general. This implies that any process can read any application binary (well, system-registered app)

The handle server could have mappings of application names to root binaries (or basically a $PATH it used). This allows running `/:Apps/file_viewer` and getting the file browser whoever you are.
- Handle server would handle various install locations.

Dynamic libraries (if any) are handled in a similar manner, except the loader also looks in `AppBin`

NOTE: Root is _not_ avaliable unless operating in an emulation environment
- Even then, it's likely a fake root


File Editing
------------
Applications opening files for editing have to ask the user (via the handle server) to select the file to open.
- If they already have a path/name, that can be passed as a hint. The server can choose to accept that hint.


Temporary Directories
---------------------

Temporaries are handled by asking the handle server for temp file or directory


Shell Emulation
---------------

In shell emulation cases (when running a program from the commandline program, or when invoking a compiler) just a Root handle is avalibale (exposed as /)

