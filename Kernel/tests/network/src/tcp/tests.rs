//! TCP tests
use crate::ipv4::Addr as IpAddr4;
use super::*;

/// TCP State CLOSED
/// 
/// Check that RST is sent when communicating with a closed port
#[test]
fn resets()
{
    const REMOTE_ADDR: IpAddr4 = IpAddr4([192,168,1,1]);
    const LOCAL_ADDR: IpAddr4 = IpAddr4([192,168,1,2]);

    let fw = {
        let mut fw = crate::TestFramework::new("tcp_resets");
        fw.add_handler(crate::arp::ArpHandler::new(LOCAL_ADDR));
        fw
        };
    
    let conn = TcpConn {
        fw: &fw,
        addrs: (LOCAL_ADDR, REMOTE_ADDR),
        remote_port: 80,
        local_port: 11200,

        rx_window: 0x1000,

        local_seq: 0x1000,
        remote_seq: 0x1000,
        };

    // SYN to closed port
    conn.raw_send_packet(TCP_SYN, &[], &[]);
    conn.wait_rx_check(TCP_RST|TCP_ACK, &[]);

    // SYN,ACK to closed port
    conn.raw_send_packet(TCP_SYN|TCP_ACK, &[], &[]);
    conn.wait_rx_check(TCP_ACK|TCP_RST, &[]);
    
    // RST to anything
    conn.raw_send_packet(TCP_RST, &[], &[]);
    conn.wait_rx_none();
    
    // RST,ACK to anything
    conn.raw_send_packet(TCP_RST|TCP_ACK, &[], &[]);
    conn.wait_rx_none();
}

/// TCP State LISTEN
#[test]
fn server()
{
    const REMOTE_ADDR: IpAddr4 = IpAddr4([192,168,1,1]);
    const LOCAL_ADDR: IpAddr4 = IpAddr4([192,168,1,2]);

    let fw = {
        let mut fw = crate::TestFramework::new("tcp_server");
        fw.add_handler(crate::arp::ArpHandler::new(LOCAL_ADDR));
        fw
        };
    fw.send_command("tcp-listen 0 80");

    let mut conn = TcpConn {
        fw: &fw,
        addrs: (LOCAL_ADDR, REMOTE_ADDR),
        remote_port: 80,
        local_port: 11200,

        rx_window: 0x1000,

        local_seq: 0x1000,
        remote_seq: 0x1000,
        };

    // >> STATE: LISTEN

    // Send RST, expect no response
    conn.raw_send_packet(TCP_RST, &[], &[]);
    conn.wait_rx_none();
    
    // Send an ACK, expect RST
    conn.raw_send_packet(TCP_ACK, &[], &[]);
    conn.wait_rx_check(TCP_ACK|TCP_RST, &[]);

    // --- Begin connection handshake --
    // - Send SYN, expect SYN,ACK
    conn.raw_send_packet(TCP_SYN, &[], &[]);
    conn.local_seq = conn.local_seq.wrapping_add(1);
    let hdr = conn.wait_rx_check(TCP_SYN|TCP_ACK, &[]);
    assert_eq!(hdr.ack, conn.local_seq, "ACK number doens't match expected");
    conn.remote_seq = hdr.seq;//.wrapping_add(1);

    // >> STATE: SYN-RECEIVED

    // - Send ACK
    conn.raw_send_packet(TCP_ACK, &[], &[]);
    conn.wait_rx_none();
    fw.send_command("tcp-accept 0 0");

    // >>> STATE: ESTABLISHED

    // Send a blob of test data
    let testblob = b"HelloWorld, this is some random testing data for TCP\xFF\x00\x66\x12\x12.";
    conn.raw_send_packet(TCP_ACK|TCP_PSH, &[], testblob);
    conn.local_seq += testblob.len() as u32;
    fw.send_command( &format!("tcp-recv-assert 0 {} {}", testblob.len(), HexString(testblob)) );

    fw.send_command( &format!("tcp-send 0 {}", HexString(testblob)) );
    conn.wait_rx_check(TCP_ACK/*|TCP_PSH*/, testblob);
    conn.remote_seq += testblob.len() as u32;
}

#[test]
fn client()
{
    let my_ip = IpAddr4([192,168,1,2]);
    
    let fw = {
        let mut fw = crate::TestFramework::new("tcp_client");
        fw.add_handler(crate::arp::ArpHandler::new(my_ip));
        fw
        };
    //crate::ipv4::prime_arp(&fw, /*dst=*/IpAddr4([192,168,1,1]), /*src=*/my_ip);

    fw.send_command(&format!("tcp-connect 0 {my_ip} 80"));
    // TODO: Expect an ARP request?

    // Expects the SYN
    let mut conn = TcpConn::from_rx_conn(&fw, 80, IpAddr4([192,168,1,2]));
    // Send SYN,ACK
    conn.raw_send_packet(TCP_SYN|TCP_ACK, &[], &[]);
    conn.local_seq += 1;
    // Expect ACK
    conn.wait_rx_check(TCP_ACK, &[]);
    // Get the client to send data
    fw.send_command("tcp-send 0 \"00 01 02 03\"");
    conn.wait_rx_check(if cfg!(feature="lwip") { TCP_ACK|TCP_PSH } else { 0 }, &[0,1,2,3]);
}

/// Helper to create a string of hex-encoded bytes
struct HexString<'a>(&'a [u8]);
impl ::std::fmt::Display for HexString<'_> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        for b in self.0 {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}