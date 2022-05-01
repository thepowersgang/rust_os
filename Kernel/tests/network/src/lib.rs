/*!
 */
use std::time::Duration;

const REMOTE_MAC: [u8; 6] = *b"RSK\x12\x34\x56";
const LOCAL_MAC: [u8; 6] = *b"RSK\xFE\xFE\xFE";

pub mod tcp;
pub mod ipv4;
pub mod ethernet;
pub mod arp;

fn bc_opts() -> impl ::bincode::Options {
	use ::bincode::Options;
    ::bincode::options().with_big_endian().allow_trailing_bytes().with_fixint_encoding()
}
fn des_be<T: for<'a> ::serde::Deserialize<'a>>(reader: &mut impl ::std::io::Read) -> ::bincode::Result<T> {
	use ::bincode::Options;
	bc_opts().deserialize_from(reader)
}
fn ser_be<T: ::serde::Serialize>(mut writer: impl ::std::io::Write, v: &T) {
    use ::bincode::Options;
    bc_opts().serialize_into(&mut writer, v).unwrap();
    //::bincode::config().big_endian().serialize_into(&mut writer, v).unwrap();
}

pub struct TestFramework {
    _lh: Option<::std::sync::MutexGuard<'static, ()>>,
    socket: std::net::UdpSocket,
    remote_addr: std::net::SocketAddr,
    process: Option<::std::process::Child>,
    logfile: std::path::PathBuf,

    cache: ::std::cell::RefCell<Cache>,
}
pub trait PacketHandler {
    /// Returns `true` is the packet was handled by the handler
    fn check_packet(&mut self, fw: &TestFramework, data: &[u8]) -> bool;
}
#[derive(Default)]
struct Cache {
    cmd_message: Option<Vec<u8>>,
    packets: ::std::collections::VecDeque< Vec<u8> >,
    handlers: Vec<Box<dyn PacketHandler>>,
}
impl TestFramework
{
    pub fn new(name: &str) -> TestFramework
    {
        ::lazy_static::lazy_static! {
            static ref LOCK: ::std::sync::Mutex<()> = ::std::sync::Mutex::new( () );
        }

        let lh = Some( LOCK.lock().unwrap_or_else(|v| v.into_inner()) );
        
		// NOTE: Ports allocated seqentially to avoid collisions between threaded tests
		static NEXT_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(12340);
        let port = NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let socket = std::net::UdpSocket::bind( ("127.0.0.1", port) ).expect("Unable to bind socket");
        let socket_str = format!("127.0.0.1:{}", port);
        
        let remote_ip = "192.168.1.1";
        let logfile: std::path::PathBuf = format!("{}.txt", name).into();

        fn spawn_host(logfile: &::std::path::Path, socket_addr: &str, remote_ip: &str, socket: &::std::net::UdpSocket, host_binname: &str) -> (Option<::std::process::Child>, ::std::net::SocketAddr)
        {

            match std::process::Command::new( env!("CARGO") )
                .arg("build").arg("--bin").arg(host_binname)
                .arg("--quiet")
                .spawn().unwrap().wait()
            {
            Ok(status) if status.success() => {},
            Ok(rc) => panic!("Building helper failed: Non-zero exit status - {}", rc),
            Err(e) => panic!("Building helper failed: {}", e),
            }
    
            println!("Spawning child");
            let mut child = std::process::Command::new( env!("CARGO") ).arg("run").arg("--quiet").arg("--bin").arg(host_binname).arg("--")
            //let mut child = std::process::Command::new("target/debug/host")
                .arg(socket_addr)
                .arg(remote_ip)// /24")
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
            (Some(child), addr)
        }


        let (child, addr) = match ::std::env::var("KNTEST_HOST").as_deref()
            {
            Ok("")
            | Err(_)
            => spawn_host(&logfile, &socket_str, remote_ip, &socket, "host"),
            Ok("lwip")
            => spawn_host(&logfile, &socket_str, remote_ip, &socket, "host_lwip"),
            Ok("none") => {
                println!("{:?}", socket_str);
                socket.set_read_timeout(None).unwrap();
                let addr = match socket.recv_from(&mut [0])
                    {
                    Ok( (_len, v) ) => v,
                    Err(e) => panic!("Child didn't connect: {}", e),
                    };
                (None, addr)
                }
            Ok(_) => panic!("Unknown host binary name"),
            };

        TestFramework {
            _lh: lh,
            socket: socket,
            remote_addr: addr,
            process: child,
            logfile: logfile,

            cache: Default::default(),
        }
    }

    pub fn add_handler(&mut self, h: impl PacketHandler + 'static) {
        self.cache.get_mut().handlers.push(Box::new(h));
    }

	pub fn send_command(&self, s: &str)
	{
		let mut msg_buf = [0; 4 + 1500];
		msg_buf[4..][..s.len()].copy_from_slice( s.as_bytes() );
		self.socket.send_to(&msg_buf[.. 4 + s.len()], self.remote_addr).expect("Failed to send to child");
	}

    fn send_packet(&self, nic: usize, ethernet_hdr: &[u8], buffers: &[ &[u8] ])
    {
        let buf: Vec<u8> = Iterator::chain([&u32::to_le_bytes(nic as u32)[..], ethernet_hdr].iter(), buffers.iter())
            .flat_map(|v| v.iter())
            .copied()
            .collect()
            ;
		println!("TX #{} {:?}", nic, HexDump(&buf[4..]));
        self.socket.send_to(&buf, self.remote_addr).expect("Failed to send to child");
    }

    /// Encode+send an ethernet frame to the virtualised NIC (addressed correctly)
    pub fn send_ethernet_direct(&self, proto: u16, buffers: &[ &[u8] ])
    {
        let ethernet_hdr = crate::ethernet::EthernetHeader { dst: REMOTE_MAC, src: LOCAL_MAC, proto: proto, }.encode();
        self.send_packet(1, &ethernet_hdr, buffers);
    }

    pub fn check_messages(&self, timeout: ::std::time::Instant) -> Option<()> {
        match timeout.checked_duration_since(::std::time::Instant::now()) {
        None => None,
        Some(d) => {
            self.socket.set_read_timeout(Some(d)).expect("Zero timeout requested");

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
                    panic!("");
                }
                if len < 4 {
                    panic!("");
                }
                buf.truncate(len);

                let idx = u32::from_le_bytes(::std::convert::TryInto::try_into(&buf[..4]).unwrap());
                let mut c = self.cache.borrow_mut();
                match idx
                {
                0 => {
                    c.cmd_message = Some(buf);
                    },
                1 => {
                    buf.drain(0..4);
                    println!("RX #{} {:?}", idx, HexDump(&buf));
                    c.packets.push_back(buf);
                    },
                _ => panic!("Unknown interface number: {}", idx),
                }
                return Some(());
            }
            }
        }
    }

    pub fn wait_packet(&self, timeout: Duration) -> Option<Vec<u8>>
    {
        let stop = ::std::time::Instant::now() + timeout;
        loop {
            let mut bh = self.cache.borrow_mut();
            if let Some(p) = bh.packets.pop_front() {
                // If none of the handlers consumed the packet, return it
                if !bh.handlers.iter_mut()
                    .any(|h| h.check_packet(self, &p))
                {
                    return Some(p);
                }
            }
            drop(bh);
            
            if let None = self.check_messages(stop) {
                return None;
            }
        }
    }
}

impl Drop for TestFramework
{
    fn drop(&mut self)
    {
        if let Some(mut process) = self.process.take()
        {
            if process.try_wait().is_err()
            {
                self.send_command("exit");
                let stop_time = ::std::time::Instant::now() + std::time::Duration::from_millis(500); // 500ms
                while process.try_wait().is_err() && ::std::time::Instant::now() < stop_time {
                    std::thread::sleep(std::time::Duration::from_millis(50) );
                }
            }
            if process.try_wait().is_err()
            {
                println!("- Child didn't respond to `exit` command, killing");
                process.kill().expect("Cannot terminate child");
            }
            process.wait().expect("Failed to wait for child");
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



pub struct ArrayBuf<const N: usize> {
    len: usize,
    inner: [u8; N],
}
impl<const N: usize> ArrayBuf<N> {
    pub fn new() -> Self {
        ArrayBuf {
            len: 0,
            inner: [0; N],
        }
    }
    pub fn extend(&mut self, i: impl IntoIterator<Item=u8>) {
        for v in i {
            self.push(v);
        }
    }
    pub fn push(&mut self, v: u8) {
        assert!(self.len < N);
        self.inner[self.len] = v;
        self.len += 1;
    }
}
impl<const N: usize> ::core::ops::Deref for ArrayBuf<N> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.inner[..self.len]
    }
}