// "Tifflin" Kernel Tests (network)
// - By John Hodge (Mutabah)
//
// tests/network/tcp.rs
//! TCP tests and infrastructure
use crate::ipv4::Addr as IpAddr4;

#[cfg(test)]
mod tests;

#[derive(Copy,Clone)]
#[derive(Debug)]
#[derive(serde_derive::Deserialize,serde_derive::Serialize)]
pub struct Header
{
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub data_ofs: u8,
    pub flags: u8,
    pub window: u16,
    pub checksum: u16,
    pub urg_ptr: u16,
}
impl Header
{
    /// Parse a TCP header, returning the options and data
    pub fn parse(mut buf: &[u8]) -> (Self, &[u8], &[u8]) {
        let rv: Self = crate::des_be(&mut buf).expect("Failed to parse TCP header");
        println!("Header: {:?}", rv);
        assert!((rv.data_ofs >> 4) >= 20/4, "Bad TCP header length");
        let option_len = (rv.data_ofs >> 4) as usize * 4 - 20;
        assert!(option_len <= buf.len(), "Bad TCP data offset: 20+{}", option_len);
        let options = &buf[..option_len];
        println!("Options: {:x?}", options);
        (rv, options, &buf[option_len..])
    }
    fn encode(&self) -> [u8; 5*4]
    {
        let mut rv = [0; 5*4];
        {
            let mut c = std::io::Cursor::new(&mut rv[..]);
            crate::ser_be(&mut c, self);
            assert_eq!(c.position(), 5*4, "Encoding of TCP header failed?");
        }
        rv
    }
    pub fn calculate_checksum_v4(&self, src: IpAddr4, dst: IpAddr4, options: &[u8], data: &[u8]) -> u16
    {
        fn u16be(a: u8, b: u8) -> u16 {
            (a as u16) << 8 | (b as u16)
        }
        fn iter_u16be<'a>(bytes: &'a [u8]) -> impl Iterator<Item=u16> + 'a {
            assert!( bytes.len() % 2 == 0 );
            bytes.chunks(2).map(|v| u16be(v[0], v[1]))
        }
        let pseudo_enc = [
            u16be(src.0[0], src.0[1]), u16be(src.0[2], src.0[3]),
            u16be(dst.0[0], dst.0[1]), u16be(dst.0[2], dst.0[3]),
            6, (20 + options.len() + data.len()) as u16,
            ];
        let it_pseudo = pseudo_enc.iter().copied();
        let hdr_enc = self.encode();
        let it_header = iter_u16be(&hdr_enc);
        let it_options = iter_u16be(options);
        let it_data = iter_u16be(data);

        crate::ipv4::calculate_ip_checksum(it_pseudo.chain(it_header).chain(it_options).chain(it_data))
    }
    pub fn set_checksum_v4(&mut self, src: IpAddr4, dst: IpAddr4, options: &[u8], data: &[u8])
    {
        self.checksum = 0;
        self.checksum = self.calculate_checksum_v4(src, dst, options, data);
    }
}
pub const TCP_FIN: u8 = 0x01;
pub const TCP_SYN: u8 = 0x02;
pub const TCP_RST: u8 = 0x04;
pub const TCP_PSH: u8 = 0x08;
pub const TCP_ACK: u8 = 0x10;

pub fn send_packet_raw(fw: &crate::TestFramework, src: IpAddr4, dst: IpAddr4, mut header: Header, options: &[u8], data: &[u8])
{
    assert!(options.len() % 4 == 0);
    header.set_checksum_v4(src, dst, options, data);
    let tcp_hdr = header.encode();
    let tcp_len = tcp_hdr.len() + options.len() + data.len();
    let ip_hdr = {
        let mut h = crate::ipv4::Header::new_simple(src, dst, 6, tcp_len);
        h.set_checksum();
        h.encode()
        };
    fw.send_ethernet_direct(0x0800, &[&ip_hdr, &tcp_hdr, options, data]);
}
/// NOTE: "local" means framework
pub struct TcpConn<'a>
{
    pub fw: &'a crate::TestFramework,
    /// Framework address, testee address
    addrs: (crate::ipv4::Addr, crate::ipv4::Addr),
    /// Testee port
    remote_port: u16, 
    /// Framework port
    local_port: u16,

    /// Framework's RX window
    pub rx_window: u16,

    pub local_seq: u32,
    pub remote_seq: u32,
}
impl TcpConn<'_>
{
    pub fn raw_send_packet(&self, flags: u8, options: &[u8], data: &[u8])
    {
        let hdr = Header {
            src_port: self.local_port,
            dst_port: self.remote_port,
            seq: self.local_seq,
            ack: self.remote_seq,
            data_ofs: ((20 + options.len() + 3)/4 << 4) as u8 | 0,
            flags: flags,
            window: self.rx_window,
            checksum: 0,
            urg_ptr: 0,
            };
        send_packet_raw(self.fw, self.addrs.0, self.addrs.1, hdr, options, data);
    }
    #[track_caller]
    pub fn wait_rx_check(&self, flags: u8, data: &[u8]) -> Header
    {
        let data_handle = match self.fw.wait_packet(std::time::Duration::from_millis(1000))
            {
            Some(v) => v,
            None => panic!("No packet recieved"),
            };
        let tail = &data_handle[..];
        // 1. Check the ethernet header
        let (ether_hdr, tail) = crate::ethernet::EthernetHeader::parse(tail);
        assert_eq!(ether_hdr.proto, 0x0800, "Incorrect ethernet protocol value: {:04x}", ether_hdr.proto);
        // 2. Check the IPv4 header
        let (ip_hdr,ip_options, tail) = crate::ipv4::Header::parse(tail);
        assert_eq!(ip_hdr.protocol, 6);
        assert_eq!(crate::ipv4::Addr(ip_hdr.src_addr), self.addrs.1);
        assert_eq!(crate::ipv4::Addr(ip_hdr.dst_addr), self.addrs.0);
        assert_eq!(ip_options.len(), 0);
        // 3. Check the TCP header (incl flags)
        let (tcp_hdr,tcp_options, tail) = Header::parse(tail);
        assert!(tcp_hdr.dst_port == self.local_port, "TCP destination port mismatch: Exp {} got {}", tcp_hdr.dst_port, self.local_port);
        assert!(tcp_hdr.src_port == self.remote_port, "TCP source port mismatch: Exp {} got {}", tcp_hdr.src_port, self.remote_port);
        //assert!(tcp_options.len() == 0, "Expected no TCP options, got {:?}", tcp_options);
        let _ = tcp_options;
        assert!(tcp_hdr.flags == flags, "Header flags mismatch: Expected {:#x} got {:#x}", flags, tcp_hdr.flags);
        // 4. Check the data
        assert_eq!(tail, data, "Data mismatch");
        tcp_hdr
    }
    #[track_caller]
    pub fn wait_rx_none(&self)
    {
        match self.fw.wait_packet(std::time::Duration::from_millis(100))
        {
        Some(_) => panic!("Unexpected packet"),
        None => {},
        }
    }

    pub fn from_rx_conn(fw: &crate::TestFramework, lport: u16, laddr: crate::ipv4::Addr) -> TcpConn
    {
        let t = std::time::Instant::now();
        let data_handle = match fw.wait_packet(std::time::Duration::from_millis(1000))
            {
            Some(v) => v,
            None => panic!("No connection packet recieved {:?}", std::time::Instant::now() - t),
            };
        let tail = &data_handle[..];
        // 1. Check the ethernet header
        let (ether_hdr, tail) = crate::ethernet::EthernetHeader::parse(tail);
        assert_eq!(ether_hdr.proto, 0x0800, "Incorrect ethernet protocol value: {:04x}", ether_hdr.proto);
        // 2. Check the IPv4 header
        let (ip_hdr,ip_options, tail) = crate::ipv4::Header::parse(tail);
        assert_eq!(ip_hdr.protocol, 6);
        //assert_eq!(crate::ipv4::Addr(ip_hdr.src_addr), self.addrs.1);
        assert_eq!(crate::ipv4::Addr(ip_hdr.dst_addr), laddr);
        assert_eq!(ip_options.len(), 0);
        // 3. Check the TCP header (incl flags)
        let (tcp_hdr,tcp_options, tail) = Header::parse(tail);
        //assert_eq!(tcp_options.len(), 0);
        let _ = tcp_options;
        assert_eq!(tcp_hdr.flags, TCP_SYN);
        assert_eq!(tcp_hdr.dst_port, lport);
        // 4. Check the data
        assert_eq!(tail, &[], "Data mismatch");
        TcpConn {
            fw: fw,
            addrs: (laddr, crate::ipv4::Addr(ip_hdr.src_addr)),
            remote_port: tcp_hdr.src_port, 
            local_port: lport,

            rx_window: 0x1000,

            local_seq: 0x10000,
            remote_seq: tcp_hdr.seq + 1,
            }
    }
}
