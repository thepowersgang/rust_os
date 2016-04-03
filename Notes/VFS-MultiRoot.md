Multiple-Root VFS Model
====

Executables get a set of directory handles, under well-known tag names

- (RO) Root?
- 
- (RO) AppBin
- (RO) AppData
- (RW) AppStorage

- (RW) File
- (RO) Input
- (RW) Output

Applications with an "application handle" (a kernel-registered non-forgable token) can make a request from a service to open a file/folder (either named or arbitary)
- This service can prompt the user, or accept the request (if whitelisted)

