
use std::mem::size_of;

#[derive(Copy,Clone)]
pub struct Addr(pub [u8; 4]);

#[derive(serde_derive::Deserialize,serde_derive::Serialize)]
pub struct Header
{
    version_and_len: u8,
    differentiated_services: u8,
    total_legnth: u16,
    identification: u16,
    fragment_info: u16,
    ttl: u8,
    protocol: u8,
    header_checksum: u16,
    src_addr: [u8; 4],
    dst_addr: [u8; 4],
}
impl Header
{
    pub fn new_simple(src: Addr, dst: Addr, proto: u8, data_len: usize) -> Self
    {
        Header {
            version_and_len: (4 << 4) | (size_of::<Header>() / 4) as u8,
            differentiated_services: 0,
            total_legnth: size_of::<Header>() as u16 + data_len as u16,
            identification: 0,
            fragment_info: 0,   // No fragments
            ttl: 18,    // Doesn't need to be high, not routed here
            protocol: proto,
            header_checksum: 0, // TODO: Calculate and populate
            src_addr: src.0,
            dst_addr: dst.0,
            }
    }
    pub fn encode(&self) -> Vec<u8>
    {
        bincode::config().big_endian().serialize(self).unwrap()
    }
}

