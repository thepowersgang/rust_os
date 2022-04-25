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

    let fw = crate::TestFramework::new("tcp_resets");
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
    conn.wait_rx_check(TCP_RST, &[]);
    
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

    let fw = crate::TestFramework::new("tcp_server");
    fw.send_command("tcp-server-echo 80");

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
    conn.wait_rx_check(TCP_RST, &[]);

    // --- Begin connection handshake --
    // - Send SYN, expect SYN,ACK
    conn.raw_send_packet(TCP_SYN, &[], &[]);
    conn.local_seq += 1;
    conn.wait_rx_check(TCP_SYN|TCP_ACK, &[]);
    conn.remote_seq += 1;

    // >> STATE: SYN-RECEIVED

    // - Send ACK
    conn.raw_send_packet(TCP_ACK, &[], &[]);
    conn.wait_rx_none();

    // >>> STATE: ESTABLISHED

    // Send a blob of test data
    let testblob = b"HelloWorld, this is some random testing data for TCP\xFF\x00\x66\x12\x12.";
    conn.raw_send_packet(TCP_ACK|TCP_PSH, &[], testblob);
    conn.local_seq += testblob.len() as u32;
    conn.wait_rx_check(TCP_ACK|TCP_PSH, testblob);
    conn.remote_seq += testblob.len() as u32;
}

#[test]
fn client()
{
    let fw = crate::TestFramework::new("tcp_client");
    crate::ipv4::prime_arp(&fw, /*dst=*/IpAddr4([192,168,1,1]), /*src=*/IpAddr4([192,168,1,2]));

    fw.send_command("tcp-connect 0 192.168.1.2 80");
    // Expects the SYN
    let conn = TcpConn::from_rx_conn(&fw, 80, IpAddr4([192,168,1,2]));
    // Send SYN,ACK
    conn.raw_send_packet(TCP_SYN|TCP_ACK, &[], &[]);
    // Expect ACK
    conn.wait_rx_check(TCP_ACK, &[]);
    // Get the client to send data
    fw.send_command("tcp-send 0 \"00 01 02 03\"");
    conn.wait_rx_check(0, &[0,1,2,3]);
}