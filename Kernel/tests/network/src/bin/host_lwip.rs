/*!
 * Network stack wrapper
 */
#[cfg(feature="lwip")]
extern crate lwip;
#[cfg(feature="lwip")]
use ::std::sync::Arc;
#[cfg(feature="lwip")]
use ::kernel_test_network::HexDump;

#[cfg(feature="lwip")]
struct Args
{
	master_addr: std::net::SocketAddr,

	sim_ip: lwip::sys::ip4_addr_t,
}

#[cfg(not(feature="lwip"))]
fn main() {
    panic!("`lwip` feature not enabled!");
}

#[cfg(feature="lwip")]
fn main()
{
	let args = {
        let mut it = std::env::args();
        it.next().unwrap();
		Args {
            master_addr: {
                let a = it.next().unwrap();
                match std::net::ToSocketAddrs::to_socket_addrs(&a)
                {
                Err(e) => panic!("Unable to parse '{}' as a socket addr: {}", a, e),
                Ok(mut v) => v.next().unwrap(),
                }
                },
			sim_ip: {
				let std_ip: std::net::Ipv4Addr = it.next().unwrap().parse().unwrap();
				let o = std_ip.octets();
                lwip::sys::ip4_addr { addr: u32::from_le_bytes(o) }
				},
			}
        };
    
    {
        let b = Arc::new(::std::sync::Barrier::new(2));
        let b2 = b.clone();
        ::lwip::os_mode::init(move || { b2.wait(); });
        b.wait();
    }
        
    let stream = match ::std::net::UdpSocket::bind("0.0.0.0:0")
        {
        Ok(v) => v,
        Err(e) => {
            println!("Cannot connect to server: {}", e);
            return
            },
        };
	stream.connect( args.master_addr ).expect("Unable to connect");
	// - Set a timeout, in case the parent fails
	stream.set_read_timeout(Some(::std::time::Duration::from_secs(1))).expect("Unable to set read timeout");
	let stream = Arc::new(stream);
    stream.send(&[0]).expect("Unable to send marker to server");


    let (tx,rx) = ::std::sync::mpsc::channel();
    ::std::thread::spawn(move || {
        // TODO: Make this a command instead
        let mac = *b"RSK\x12\x34\x56";
        let nic_handle = TestNicHandle::new( stream.clone(), mac, args.sim_ip, 24);

        loop
        {
            const MTU: usize = 1560;
            let mut buf = [0; 4 + MTU];
            let len = match stream.recv(&mut buf)
                {
                Ok(len) => len,
                Err(e) => {
                    println!("Error receiving packet: {:?}", e);
                    break;
                    },
                };
            if len == 0 {
                println!("ERROR: Zero-sized packet?");
                break;
            }
            if len < 4 {
                println!("ERROR: Runt packet {:?}", HexDump(&buf[..len]));
                break;
            }
            let id = u32::from_le_bytes(std::convert::TryInto::try_into(&buf[..4]).unwrap());
            let data = &mut buf[4..len];
            if id == 0
            {
                let line = match std::str::from_utf8(data)
                    {
                    Ok(v) => v,
                    Err(e) => panic!("Bad UTF-8 from server: {:?} - {:?}", e, HexDump(&data)),
                    };
                println!("COMMAND {:?}", line);

                tx.send(line.to_owned()).expect("Failed to send command to main thread");
            }
            else
            {
                let buf = data.to_owned();
                println!("RX #{} {:?}", id, HexDump(data));
                let nic = match id
                    {
                    0 => unreachable!(),
                    1 => &nic_handle,
                    _ => panic!("Unknown NIC ID {}", id),
                    };
                nic.packet_received(buf);
            }
        }
        });

	// Monitor for commands
	let mut tcp_conn_handles = ::std::collections::HashMap::new();
	let mut tcp_server_handles = ::std::collections::HashMap::new();
    for mut line in rx
    {
        let mut it = ::cmdline_words_parser::parse_posix(&mut line[..]);
        let cmd = match it.next()
            {
            Some(c) => c,
            None => {
                println!("stdin empty");
                break
                },
            };
        match cmd
        {
        "" => {},
        "exit" => {
            println!("exit command");
            break
            },
        "ipv4-add" => {
            },
        // Listen on a port/interface
        "tcp-listen" => {
            let index: usize = it.next().unwrap().parse().unwrap();
            let port : u16   = it.next().unwrap().parse().unwrap();
            println!("tcp-listen {} = *:{}", index, port);
            tcp_server_handles.insert(index, ::lwip::netconn::TcpServer::listen_with_backlog(port, 2).unwrap());
            println!("OK");
            },
        "tcp-accept" => {
            let c_index: usize = it.next().unwrap().parse().unwrap();
            let s_index: usize = it.next().unwrap().parse().unwrap();
            println!("tcp-accept {} = [{}]", c_index, s_index);
            let s = tcp_server_handles.get_mut(&s_index).expect("BUG: Bad server index");
            tcp_conn_handles.insert(c_index, ClientSocket::from_conn(s.accept().expect("No waiting connection")));
            println!("OK");
            },
        // Make a connection
        "tcp-connect" => {
            // Get dest ip & dest port
            let index: usize = it.next().unwrap().parse().unwrap();
            let ip: ::lwip::sys::ip_addr = parse_addr(it.next().expect("Missing IP")).unwrap();
            let port: u16 = it.next().unwrap().parse().unwrap();
            println!("tcp-connect {} = {}:{}", index, ip, port);
            tcp_conn_handles.insert(index, ClientSocket::connect(ip, port).unwrap());
            println!("OK");
            },
        // Close a TCP connection
        "tcp-close" => {
            let index: usize = it.next().unwrap().parse().unwrap();
            todo!("tcp-close {}", index);
            },
        "tcp-send" => {
            let index: usize = it.next().unwrap().parse().unwrap();
            let bytes = parse_hex_bytes(it.next().unwrap()).unwrap();
            let h = &tcp_conn_handles[&index];
            println!("tcp-send {} {:?}", index, bytes);
            h.send_data(&bytes).unwrap();
            println!("OK");
            },
        "tcp-recv-assert" => {
            let index: usize = it.next().unwrap().parse().unwrap();
            let read_size: usize = it.next().unwrap().parse().unwrap();
            let exp_bytes = parse_hex_bytes(it.next().unwrap()).unwrap();
            // - Receive bytes, check that they equal an expected value
            // NOTE: No wait
            println!("tcp-recv-assert {} {} == {:?}", index, read_size, exp_bytes);
            let h = tcp_conn_handles.get_mut(&index).unwrap();

            let mut buf = vec![0; read_size];
            let len = h.recv_data(&mut buf).unwrap();
            assert_eq!(&buf[..len], &exp_bytes[..]);
            println!("OK");
            },
        _ => eprintln!("ERROR: Unknown command '{}'", cmd),
        }
    }
}

#[cfg(feature="lwip")]
fn parse_hex_bytes(s: &str) -> Option<Vec<u8>>
{
	let mut nibble = 0;
	let mut cur_byte = 0;
	let mut rv = Vec::new();
	for c in s.chars()
	{
		if c.is_whitespace() {
			continue ;
		}
		let d = c.to_digit(16)?;

		cur_byte |= d << (4 * (1 - nibble));
		nibble += 1;

		if nibble == 2 {
			rv.push(cur_byte as u8);
			cur_byte = 0;
			nibble = 0;
		}
	}

	if nibble != 0 {
		None
	}
	else {
		Some(rv)
	}
}

#[cfg(feature="lwip")]
fn parse_addr(s: &str) -> Option<::lwip::sys::ip_addr>
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
        Some(::lwip::sys::ip_addr {
            type_: ::lwip::sys::lwip_ip_addr_type_IPADDR_TYPE_V4 as u8,
            u_addr: ::lwip::sys::ip_addr__bindgen_ty_1 {
                ip4: ::lwip::sys::ip4_addr { addr: u32::from_le_bytes([b1,b2,b3,b4]) },
            },
        })
	}
	else {
		None
	}
}

#[cfg(feature="lwip")]
struct TestNicHandle
{
    index: u32,
    stream: Arc<std::net::UdpSocket>,
    mac: [u8; 6],
    netif: ::std::cell::UnsafeCell<::lwip::sys::netif>,
}
#[cfg(feature="lwip")]
impl TestNicHandle
{
    fn new(stream: Arc<std::net::UdpSocket>, mac: [u8; 6], ip: ::lwip::sys::ip4_addr_t, mask_bits: u8) -> &'static TestNicHandle {
        let rv = Box::new(TestNicHandle {
            index: 1,
            stream,
            mac,
            netif: ::std::cell::UnsafeCell::new(unsafe { ::core::mem::zeroed() }),
            });
        let mask = ::lwip::sys::ip4_addr_t { addr: (!0u32 << (32 - mask_bits)).swap_bytes() };
        let gw = ::lwip::sys::ip4_addr_t { addr: 0xC0A80102u32.swap_bytes() };
        println!("TestNicHandle: {} {} gw {}", ip, mask, gw);
        unsafe {
            let netif_ptr = rv.netif.get();
            let state_ptr = &*rv as *const _ as *mut ::std::ffi::c_void;
            ::lwip::os_mode::callback(move || {
                let rv = ::lwip::sys::netif_add(
                    netif_ptr, &ip, &mask, &gw,
                    state_ptr, Some(Self::init), Some(::lwip::sys::tcpip_input)
                    );
                println!("Added! {:p} == {:p}", rv, netif_ptr);
            })
        }
        Box::leak(rv)
    }

    unsafe extern "C" fn init(netif_r: *mut ::lwip::sys::netif) -> ::lwip::sys::err_t {
        let netif = &mut *netif_r;
        let this = &*(netif.state as *const TestNicHandle);
        netif.hwaddr_len = 6;
        netif.hwaddr = this.mac;
        netif.mtu = 1520;
        netif.flags = 0    
            | ::lwip::sys::NETIF_FLAG_BROADCAST as u8   // Broadcast allowed
            | ::lwip::sys::NETIF_FLAG_LINK_UP as u8 // The link is always up
            | ::lwip::sys::NETIF_FLAG_ETHERNET as u8    // Ethernet
            | ::lwip::sys::NETIF_FLAG_ETHARP as u8  // With ARP/IP (i.e. not PPPoE)
            ;
        netif.linkoutput = Some(Self::linkoutput);
        netif.output = Some(Self::etharp_output);
        ::lwip::sys::netif_set_link_up(netif_r);
        ::lwip::sys::netif_set_up(netif_r);
        ::lwip::sys::netif_set_default(netif_r);
        // Do anything?
        //println!("Init done {:p} {:p} {:#x} {:x?}", netif_r, netif.state, netif.flags, netif.hwaddr);
        //println!("- linkoutput = {:?}, {:p}", netif.linkoutput, Self::linkoutput as unsafe extern "C" fn(_,_)->_);
        //println!("- output = {:?}, {:p}", netif.output, Self::etharp_output as unsafe extern "C" fn(_,_,_)->_);
        ::lwip::sys::err_enum_t_ERR_OK as i8
    }

    fn packet_received(&self, buf: Vec<u8>) {
        let _ = buf;
        let pbuf = unsafe { ::lwip::sys::pbuf_alloc(buf.len() as u32, buf.len() as u16, ::lwip::sys::pbuf_type_PBUF_RAM) };
        unsafe { ::core::ptr::copy_nonoverlapping(buf.as_ptr(), (*pbuf).payload as *mut _, buf.len()); }
        let input_fcn = unsafe { (&*self.netif.get()).input.unwrap() };
        unsafe { input_fcn(pbuf, self.netif.get()); }
    }

    unsafe extern "C" fn etharp_output(netif: *mut ::lwip::sys::netif, pbuf: *mut ::lwip::sys::pbuf, ipaddr: *const ::lwip::sys::ip4_addr_t) -> ::lwip::sys::err_t {
        ::lwip::sys::etharp_output(netif, pbuf, ipaddr)
    }

    unsafe extern "C" fn linkoutput(this_r: *mut ::lwip::sys::netif, pbuf: *mut ::lwip::sys::pbuf) -> ::lwip::sys::err_t {
        let this = &*((*this_r).state as *const TestNicHandle);
        
        let buf = {
            let mut buf = Vec::new();
            buf.extend(this.index.to_le_bytes());
            let mut pbuf = pbuf;
            while !pbuf.is_null() {
                pbuf = {
                    let pbuf = &*pbuf;
                    let d = ::std::slice::from_raw_parts(pbuf.payload as *const u8, pbuf.len as usize);
                    buf.extend(d.iter().copied());
                    pbuf.next
                    };
            }
            buf
            };

		println!("TX #{} {:?}", this.index, HexDump(&buf[4..]));
        this.stream.send(&buf).unwrap();

        ::lwip::sys::err_enum_t_ERR_OK as i8
    }
}


#[cfg(feature="lwip")]
struct ClientSocket {
    conn: ::lwip::netconn::TcpConnection,
    cur_buf: Option<::lwip::netconn::Netbuf>,
    cur_ofs: usize,
}
#[cfg(feature="lwip")]
impl ClientSocket {
    fn from_conn(conn: ::lwip::netconn::TcpConnection) -> Self {
        ClientSocket { conn, cur_buf: None, cur_ofs: 0 }
    }
    pub fn connect(ip: ::lwip::sys::ip_addr, port: u16) -> Result<Self,::lwip::Error> {
        Ok(Self::from_conn( ::lwip::netconn::TcpConnection::connect(ip, port)? ))
    }

    pub fn send_data(&self, bytes: &[u8]) -> Result<usize,::lwip::Error> {
        self.conn.send(bytes)
    }
    pub fn recv_data(&mut self, buf: &mut [u8]) -> Result<usize,::lwip::Error> {

        fn partial_read(dst: &mut [u8], dst_ofs: &mut usize, src: &::lwip::netconn::Netbuf, src_ofs: &mut usize) -> Result<(), ::lwip::Error>
        {
            let src = src.get_slice()?;
            assert!(*src_ofs < src.len());
            assert!(*dst_ofs <= dst.len());
            let l = ::std::cmp::Ord::min(src.len() - *src_ofs, dst.len());

            dst[*dst_ofs..][..l].copy_from_slice(&src[*src_ofs..][..l]);
            *src_ofs += l;
            *dst_ofs += l;
            assert!(*src_ofs <= src.len());
            assert!(*dst_ofs <= dst.len());
            Ok( () )
        }

        let mut buf_ofs = 0;

        // If there's data from a previous read attempt, read from there first.
        if let Some(ref src) = self.cur_buf
        {
            partial_read(buf, &mut buf_ofs, src, &mut self.cur_ofs)?;
            
            if buf_ofs == buf.len() {
                self.cur_buf = None;
                self.cur_ofs = 0;
            }
            else {
                return Ok(buf_ofs);
            }
        }

        let inbuf = self.conn.recv()?;
        partial_read(buf, &mut buf_ofs, &inbuf, &mut self.cur_ofs)?;
        
        // If the output buffer consumed fully, then there is still data in the input buffer (or it's empty)
        if buf_ofs == buf.len() {
            self.cur_buf = Some(inbuf);
        }

        Ok(buf_ofs)
    }
}
