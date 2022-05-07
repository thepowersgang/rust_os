/*!
 * Network stack wrapper
 */
#[macro_use]
extern crate kernel;

use ::kernel_test_network::HexDump;
use std::sync::Arc;

#[cfg(not(feature="lwip"))]
mod backend_kernel;
#[cfg(not(feature="lwip"))]
use self::backend_kernel as backend;
#[cfg(feature="lwip")]
mod backend_lwip;
#[cfg(feature="lwip")]
use self::backend_lwip as backend;

struct Args
{
	master_addr: std::net::SocketAddr,

	sim_ip: backend::IpAddr,
}

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
			sim_ip: backend::parse_addr( &it.next().unwrap() ).unwrap(),
			}
        };
    
	backend::init();
        
    let stream = match std::net::UdpSocket::bind("0.0.0.0:0")
        {
        Ok(v) => v,
        Err(e) => {
            println!("Cannot connect to server: {}", e);
            return
            },
        };
	stream.connect( args.master_addr ).expect("Unable to connect");
	// - Set a timeout, in case the parent fails (semi-ensures that the process quits if the test framework dies)
	stream.set_read_timeout(Some(::std::time::Duration::from_secs(10))).expect("Unable to set read timeout");
	let stream = Arc::new(stream);
    stream.send(&[0]).expect("Unable to send marker to server");

    let (tx,rx) = ::std::sync::mpsc::channel();
	backend::spawn_thread(move || {
		let nic_handle = backend::create_interface(stream.clone(), 1, *b"RSK\x12\x34\x56", args.sim_ip);
		
        loop
        {
            const MTU: usize = 1560;
            let mut buf = [0; 4 + MTU];
            let len = match backend::run_blocking(|| stream.recv(&mut buf))
                {
                Ok(len) => len,
                Err(e) => panic!("Error receiving packet: {:?}", e),
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
                println!("> ENQUEUE COMMAND {:?}", line);

                tx.send(line.to_owned()).expect("Failed to send command to main thread");
            }
            else
            {
                let buf = data.to_owned();
                match id
				{
				0 => unreachable!(),
				1 => nic_handle.packet_received(buf),
				_ => panic!("Unknown NIC ID {}", id),
				}
            }
        }
        });


	// Monitor stdin for commands
	let mut tcp_conn_handles = ::std::collections::HashMap::new();
	let mut tcp_server_handles = ::std::collections::HashMap::new();
	
    loop
    {
		let mut line = backend::run_blocking(|| rx.recv()).unwrap();

		let mut it = ::cmdline_words_parser::parse_posix(&mut line[..]);
		let cmd = match it.next()
			{
			Some(c) => c,
			None => {
				log_notice!("stdin empty");
				break
				},
			};
		match cmd
		{
		"" => {},
		"exit" => {
			log_notice!("exit command");
			break
			},
		"ipv4-add" => {
			},
		// Listen on a port/interface
		"tcp-listen" => {
			let index: usize = it.next().unwrap().parse().unwrap();
			let port : u16   = it.next().unwrap().parse().unwrap();
			log_notice!("tcp-listen {} = *:{}", index, port);
			tcp_server_handles.insert(index, backend::tcp_listen(port));
			println!("OK");
			},
		"tcp-accept" => {
			let c_index: usize = it.next().unwrap().parse().unwrap();
			let s_index: usize = it.next().unwrap().parse().unwrap();
			log_notice!("tcp-accept {} = [{}]", c_index, s_index);
			let s = tcp_server_handles.get_mut(&s_index).expect("BUG: Bad server index");
			tcp_conn_handles.insert(c_index, s.accept().expect("No waiting connection"));
			println!("OK");
			},
		// Make a connection
		"tcp-connect" => {
			// Get dest ip & dest port
			let index: usize = it.next().unwrap().parse().unwrap();
			let ip = backend::parse_addr(it.next().expect("Missing IP")).unwrap();
			let port: u16 = it.next().unwrap().parse().unwrap();
			log_notice!("tcp-connect {} = {:?}:{}", index, ip, port);
			tcp_conn_handles.insert(index, backend::tcp_connect(ip, port));
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
			log_notice!("tcp-send {} {:?}", index, bytes);
			h.send_data(&bytes).unwrap();
			println!("OK");
			},
		"tcp-recv-assert" => {
			let index: usize = it.next().unwrap().parse().unwrap();
			let read_size: usize = it.next().unwrap().parse().unwrap();
			let exp_bytes = parse_hex_bytes(it.next().unwrap()).unwrap();
			// - Receive bytes, check that they equal an expected value
			// NOTE: No wait
			log_notice!("tcp-recv-assert {} {} == {:?}", index, read_size, exp_bytes);
			let h = tcp_conn_handles.get_mut(&index).unwrap();

			let mut buf = vec![0; read_size];
			let len = h.recv_data(&mut buf).unwrap();
			assert_eq!(&buf[..len], &exp_bytes[..]);
			println!("OK");
			},
		_ => panic!("ERROR: Unknown command '{}'", cmd),
		}
    }
}

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
