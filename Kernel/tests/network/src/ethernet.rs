

#[derive(serde_derive::Deserialize,serde_derive::Serialize)]
pub struct EthernetHeader
{
    pub dst: [u8; 6],
    pub src: [u8; 6],
    pub proto: u16,
}
impl EthernetHeader
{
    pub fn parse(mut buf: &[u8]) -> (Self, &[u8]) {
        let rv: Self = bincode::config().big_endian().deserialize_from(&mut buf).expect("Failed to parse ethernet header");
        (rv, buf)
    }
    pub fn encode(&self) -> [u8; 6+6+2] {
        let mut rv = [0; 14];
        {
            let mut c = std::io::Cursor::new(&mut rv[..]);
            bincode::config().big_endian().serialize_into(&mut c, self).unwrap();
            assert!(c.position() == 14);
        }
        rv
    }
}