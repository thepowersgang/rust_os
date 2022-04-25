

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
        let rv: Self = crate::des_be(&mut buf).expect("Failed to parse ethernet header");
        (rv, buf)
    }
    pub fn encode(&self) -> [u8; 6+6+2] {
        let mut rv = [0; 14];
        {
            let mut c = std::io::Cursor::new(&mut rv[..]);
            crate::ser_be(&mut c, self);
            assert!(c.position() == 14);
        }
        rv
    }
}