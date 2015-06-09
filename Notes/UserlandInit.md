Initial userland program (basically a lightweight init) is loaded from a custom binary format


# Init Format
- 16-byte header
 - Magic number (32-bits)
 - Entrypoint (32-bit even on 64-bit platforms, magic includes target arch).
  > This also defines the load address, as the entrypoint must be in the first 4KB of the binary)
 - Code size (i.e. how many bytes need to be RO+X)
 - Reserve size (i.e. how large this image is in-memory, will be padded with writable zeroes)
