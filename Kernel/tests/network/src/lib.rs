/*!
 */
use std::time::Duration;

const REMOTE_MAC: [u8; 6] = *b"RSK\x12\x34\x56";
const LOCAL_MAC: [u8; 6] = *b"RSK\xFE\xFE\xFE";

pub mod tcp;
pub mod ipv4;
pub mod ethernet;

pub struct TestFramework {
    socket: std::net::UdpSocket,
    remote_addr: std::net::SocketAddr,
    process: std::process::Child,
    logfile: std::path::PathBuf,
}
impl TestFramework
{
    pub fn new(name: &str) -> TestFramework
    {
        let logfile: std::path::PathBuf = format!("{}.txt", name).into();
        let port = 1234;

        match std::process::Command::new( env!("CARGO") )
            .arg("build").arg("--bin").arg("host")
            .spawn().unwrap().wait()
        {
        Ok(status) if status.success() => {},
        Ok(rc) => panic!("Building helper failed: Non-zero exit status - {}", rc),
        Err(e) => panic!("Building helper failed: {}", e),
        }


        let socket = std::net::UdpSocket::bind( ("127.0.0.1", port) ).expect("Unable to bind socket");
        println!("Spawning child");
        let mut child = std::process::Command::new( env!("CARGO") )
            .arg("run").arg("--quiet").arg("--bin").arg("host")
            .arg("--")
            .arg(format!("127.0.0.1:{}", port))
            .arg("192.168.1.1")// /24")
            .stdout(std::fs::File::create(&logfile).unwrap())
            //.stderr(std::fs::File::create("stderr.txt").unwrap())
            .spawn()
            .expect("Can't spawn child")
            ;
        println!("Waiting for child");
        socket.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let addr = match socket.recv_from(&mut [0])
            {
            Ok( (_len, v) ) => v,
            Err(e) => {
                match child.try_wait()
                {
                Ok(_) => {},
                Err(_) => child.kill().expect("Unable to terminate child"),
                }
                panic!("Child didn't connect: {}", e)
                },
            };

        TestFramework {
            socket: socket,
            remote_addr: addr,
            process: child,
            logfile: logfile,
        }
    }

    /// Encode+send an ethernet frame to the virtualised NIC (addressed correctly)
    pub fn send_ethernet_direct(&self, proto: u16, buffers: &[ &[u8] ])
    {
        let ethernet_hdr = crate::ethernet::EthernetHeader { dst: REMOTE_MAC, src: LOCAL_MAC, proto: proto, }.encode();
        let buf: Vec<u8> = Iterator::chain([&ethernet_hdr as &[u8]].iter(), buffers.iter())
            .flat_map(|v| v.iter())
            .copied()
            .collect()
            ;
        self.socket.send_to(&buf, self.remote_addr).expect("Failed to send to child");
    }

    pub fn wait_packet(&self, timeout: Duration) -> Option<Vec<u8>>
    {
        self.socket.set_read_timeout(Some(timeout)).expect("Zero timeout requested");
        let mut buf = vec![0; 1560];
        let (len, addr) = match self.socket.recv_from(&mut buf)
            {
            Ok(v) => v,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return None,
            Err(e) => panic!("wait_packet: Error {}", e),
            };
        if addr != self.remote_addr {
            // Hmm...
        }
        buf.truncate(len);
        Some(buf)
    }
}

impl Drop for TestFramework
{
    fn drop(&mut self)
    {
        self.process.kill().expect("Cannot terminate child");
        if std::thread::panicking() {
            println!("See {} for worker log", self.logfile.display());
        }
    }
}