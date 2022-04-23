/*!
 */
use std::time::Duration;

const REMOTE_MAC: [u8; 6] = *b"RSK\x12\x34\x56";
const LOCAL_MAC: [u8; 6] = *b"RSK\xFE\xFE\xFE";

pub mod tcp;
pub mod ipv4;
pub mod ethernet;

pub struct TestFramework {
    _lh: Option<::std::sync::MutexGuard<'static, ()>>,
    socket: std::net::UdpSocket,
    remote_addr: std::net::SocketAddr,
    process: std::process::Child,
    logfile: std::path::PathBuf,
}
impl TestFramework
{
    pub fn new(name: &str) -> TestFramework
    {
        ::lazy_static::lazy_static! {
            static ref LOCK: ::std::sync::Mutex<()> = ::std::sync::Mutex::new( () );
        }

        let lh = Some( LOCK.lock().unwrap_or_else(|v| v.into_inner()) );

        let logfile: std::path::PathBuf = format!("{}.txt", name).into();
		// NOTE: Ports allocated seqentially to avoid collisions between threaded tests
		static NEXT_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(12340);
        let port = NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        match std::process::Command::new( env!("CARGO") )
            .arg("build").arg("--bin").arg("host")
            .arg("--quiet")
            .spawn().unwrap().wait()
        {
        Ok(status) if status.success() => {},
        Ok(rc) => panic!("Building helper failed: Non-zero exit status - {}", rc),
        Err(e) => panic!("Building helper failed: {}", e),
        }


        let socket = std::net::UdpSocket::bind( ("127.0.0.1", port) ).expect("Unable to bind socket");
        println!("Spawning child");
        let mut child = std::process::Command::new( env!("CARGO") ).arg("run").arg("--quiet").arg("--bin").arg("host").arg("--")
        //let mut child = std::process::Command::new("target/debug/host")
            .arg(format!("127.0.0.1:{}", port))
            .arg("192.168.1.1")// /24")
			//.stdin( std::process::Stdio::piped() )
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
            _lh: lh,
            socket: socket,
            remote_addr: addr,
            process: child,
            logfile: logfile,
        }
    }

	pub fn send_command(&self, s: &str)
	{
		let mut msg_buf = [0; 4 + 1500];
		msg_buf[4..][..s.len()].copy_from_slice( s.as_bytes() );
		self.socket.send_to(&msg_buf[.. 4 + s.len()], self.remote_addr).expect("Failed to send to child");
	}

    /// Encode+send an ethernet frame to the virtualised NIC (addressed correctly)
    pub fn send_ethernet_direct(&self, proto: u16, buffers: &[ &[u8] ])
    {
        let ethernet_hdr = crate::ethernet::EthernetHeader { dst: REMOTE_MAC, src: LOCAL_MAC, proto: proto, }.encode();
        let buf: Vec<u8> = Iterator::chain([&[1,0,0,0], &ethernet_hdr as &[u8]].iter(), buffers.iter())
            .flat_map(|v| v.iter())
            .copied()
            .collect()
            ;
		println!("TX {:?}", HexDump(&buf));
        self.socket.send_to(&buf, self.remote_addr).expect("Failed to send to child");
    }

    pub fn wait_packet(&self, timeout: Duration) -> Option<Vec<u8>>
    {
        self.socket.set_read_timeout(Some(timeout)).expect("Zero timeout requested");
        let mut buf = vec![0; 1560];
		loop
		{
			let (len, addr) = match self.socket.recv_from(&mut buf)
				{
				Ok(v) => v,
				Err(e) if e.kind() == std::io::ErrorKind::TimedOut => return None,
				Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return None,
				Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
				Err(e) => panic!("wait_packet: Error {} (Kind = {:?})", e, e.kind()),
				};
			if addr != self.remote_addr {
				// Hmm...
			}
			buf.truncate(len);
			println!("RX {:?}", HexDump(&buf));
			return Some(buf);
		}
    }
}

impl Drop for TestFramework
{
    fn drop(&mut self)
    {
		if self.process.try_wait().is_err()
		{
			self.send_command("exit");
			std::thread::sleep(std::time::Duration::new(0,500*1000) );
		}
		if self.process.try_wait().is_err()
		{
			self.process.kill().expect("Cannot terminate child");
		}
        if std::thread::panicking() {
            println!("See {} for worker log", self.logfile.canonicalize().unwrap().display());
        }
    }
}


/// Wrapper around a &-ptr that prints a hexdump of the passed data.
pub struct HexDump<'a>(pub &'a [u8]);

impl<'a> HexDump<'a>
{
}

impl<'a> ::std::fmt::Debug for HexDump<'a>
{
	fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result
	{
		let slice = self.0;
		write!(f, "{} bytes: ", slice.len())?;
		for (idx,v) in slice.iter().enumerate()
		{
			write!(f, "{:02x} ", *v)?;
			if idx % 16 == 15 {
				write!(f, "| ")?;
			}
		}
		Ok( () )
	}
}
