
use ::std::sync::Arc;

mod nic;
mod client_socket;
pub use self::nic::TestNicHandle;
pub use self::client_socket::ClientSocket;

pub type IpAddr = ::lwip::sys::ip4_addr_t;

pub fn parse_addr(s: &str) -> Option<::lwip::sys::ip4_addr_t>
{
	if s.contains(".") {
		let mut it = s.split('.');
		let b1: u8 = it.next()?.parse().ok()?;
		let b2: u8 = it.next()?.parse().ok()?;
		let b3: u8 = it.next()?.parse().ok()?;
		let b4: u8 = it.next()?.parse().ok()?;
		if it.next().is_some() {
			return None;
		}
        Some( ::lwip::sys::ip4_addr { addr: u32::from_le_bytes([b1,b2,b3,b4]) } )
	}
	else {
		None
	}
}

pub fn init()
{
    let b = Arc::new(::std::sync::Barrier::new(2));
    let b2 = b.clone();
    ::lwip::os_mode::init(move || { b2.wait(); });
    b.wait();
}

pub fn create_interface(stream: Arc<::std::net::UdpSocket>, number: u32, mac: [u8; 6], addr: IpAddr) -> &'static TestNicHandle {
    TestNicHandle::new( number, stream, mac, addr, 24 )
}

pub fn spawn_thread(f: impl FnOnce() + Send + 'static) {
    ::std::thread::spawn(f);
}
pub fn run_blocking<T>(f: impl FnOnce()->T) -> T {
    f()
}

pub fn tcp_connect(ip: IpAddr, port: u16) -> client_socket::ClientSocket {
    client_socket::ClientSocket::connect(ip, port).unwrap()
}
pub fn tcp_listen(port: u16) -> Server {
    Server( ::lwip::netconn::TcpServer::listen_with_backlog(port, 2).unwrap() )
}


pub struct Server(::lwip::netconn::TcpServer);
impl Server
{
    pub fn accept(&self) -> Option<ClientSocket> {
        Some(ClientSocket::from_conn(self.0.accept()?))
    }
}
