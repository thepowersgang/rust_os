
use crate::ipv4::Addr as IpAddr4;

#[derive(serde_derive::Deserialize,serde_derive::Serialize)]
struct TcpHeader
{
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    data_ofs: u8,
    flags: u8,
    window: u16,
    checksum: u16,
    urg_ptr: u16,
}
impl TcpHeader
{
    fn encode(&self) -> [u8; 5*4]
    {
        let mut rv = [0; 5*4];
        {
            let mut c = std::io::Cursor::new(&mut rv[..]);
            bincode::config().big_endian().serialize_into(&mut c, self).unwrap();
            assert!(c.position() == 5*4);
        }
        rv
    }
}
const TCP_FIN: u8 = 0x01;
const TCP_SYN: u8 = 0x02;
const TCP_RST: u8 = 0x04;

fn send_packet_raw(fw: &crate::TestFramework, src: IpAddr4, dst: IpAddr4, header: &TcpHeader, options: &[u8], data: &[u8])
{
    assert!(options.len() & 3 == 0);
    let tcp_hdr = header.encode();
    let tcp_len = tcp_hdr.len() + options.len() + data.len();
    let ip_hdr = crate::ipv4::Header::new_simple(src, dst, 6, tcp_len).encode();
    fw.send_ethernet_direct(0x0800, &[&ip_hdr, &tcp_hdr, options, data]);
}
struct TcpConn<'a>
{
    fw: &'a crate::TestFramework,
    addrs: (crate::ipv4::Addr, crate::ipv4::Addr),
    remote_port: u16, 
    local_port: u16,

    rx_window: u16,

    local_seq: u32,
    remote_seq: u32,
}
impl TcpConn<'_>
{
    fn raw_send_packet(&self, flags: u8, options: &[u8], data: &[u8])
    {
        let hdr = TcpHeader {
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
        send_packet_raw(self.fw, self.addrs.0, self.addrs.1, &hdr, options, data);
    }
    fn wait_rx_check(&self, flags: u8, data: &[u8])
    {
        let data_handle = match self.fw.wait_packet(std::time::Duration::from_millis(1000))
            {
            Some(v) => v,
            None => panic!("No packet recieved"),
            };
    }
}

const REMOTE_ADDR: IpAddr4 = IpAddr4([192,168,1,1]);
const LOCAL_ADDR: IpAddr4 = IpAddr4([192,168,1,2]);

/// Check that RST is sent when communicating with a closed port
#[test]
fn resets()
{
    let fw = crate::TestFramework::new();
    network::ipv4::add_interface(super::REMOTE_MAC, network::ipv4::Address::new(192,168,1,1));
    let conn = TcpConn {
        fw: &fw,
        addrs: (LOCAL_ADDR, REMOTE_ADDR),
        remote_port: 80,
        local_port: 11200,

        rx_window: 0x1000,

        local_seq: 0x1000,
        remote_seq: 0x1000,
        };

    kernel::log_trace!("TEST: start");
    conn.raw_send_packet(TCP_SYN, &[], &[]);
    conn.wait_rx_check(TCP_RST, &[]);
    kernel::log_trace!("TEST: complete");
}