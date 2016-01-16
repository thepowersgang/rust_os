
Write Requirements:
- On a write request, the current handle must be made unique:
 - Block subsequent read requests
 - Wait until existing reads complete? (Would like to COW instead)
 - COW leads to possibly unbonded memory usage, and requires remembering if a read handle should drop the data

- Multiple concurrent readers

- Need to GC the mapping
 - Perform on-demand GC when out of mapping slots?
 - Or just unmap when the last reader descopes?
