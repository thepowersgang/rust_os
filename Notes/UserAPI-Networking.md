
Requirements
=======

((see http://doc.rust-lang.org/std/net/ and https://doc.rust-lang.org/net2-rs/))
- TCP Servers
 - With support for knowing the remote endpoint (ip and port)
 - Reusable port
- TCP Streams
 - Time-To-Live support (with related error)
 - NODELAY (disable in-kernel TX caching)
 - Keepalive and RX/TX timeouts
 - Non-blocking IO
- UDP Datagram sockets
 - sendto/recvfrom type API?
- SCTP Streams

- DNS lookups (userland only)
- Support for IPv4/IPv6 (and other)?
- Ping etc (allow IP-level sockets?)

- Socket Addressing:
 1. Assume IPv6/IPv4 only and use ::192.168.1.1 style encoding
 2. Use the Acess model of type+address
  * Maybe extend with type+size+address?

- Network management:
 - Address assignments (multiples per NIC)
 - NIC Properties (MAC, VLANs)
 - Routes
 - Firewalling

- NIC inerfaces
 - Fixed parameters at registration (HW MAC, Media Type)
 - Support offloading features (TSO, checksums, VLANs)
 - Config updates (e.g. changing MAC address)
 - Media sense support

Design
=======

Userland
----

* Addresses
 * NetworkAddress
  * Type (enum), Data (slice)
* Sockets:
 * StreamServer
```rust
impl StreamServer
{
	
}
```
 * DatagramServer
 * StreamSocket
 * DatagramSocket

Kernelland
----
NIC Interface:
* NOTE: Should be async capable


% vim: ft=markdown

