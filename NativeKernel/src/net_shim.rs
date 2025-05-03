//! Network simulaed using SLIRP (same library as used by qemu's user networking)
//! 
//! NOTE: Only works on unix, due SLIRP using raw socket numbers only available on unix
use ::std::sync::Arc;

pub struct Nic {
	tx_sender: Arc< ::std::os::unix::net::UnixDatagram >,
	rx_common: Arc<RxCommon>,
}
unsafe impl Send for Nic { }
unsafe impl Sync for Nic { }

struct RxCommon {
	waiter: ::kernel::threads::AtomicSleepObjectRef,
	packets: ::kernel::sync::Queue< Vec<u8> >,
	out_file: ::std::sync::Mutex<::pcap_writer::PcapWriter< ::std::fs::File >>,
}

impl Nic {
	pub fn new(mac: [u8; 6]) -> Self {
		let rx_common = ::std::sync::Arc::new(RxCommon {
			waiter: Default::default(),
			packets: Default::default(),
			out_file: ::std::sync::Mutex::new(pcap_writer::PcapWriter::new(::std::fs::File::create(
				format!("native_net_{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}.pcap",
					mac[0],mac[1],mac[2],mac[3],mac[4],mac[5],
				)
			).unwrap()).unwrap()),
		});
		let (sock_slirp, sock_outer) = ::std::os::unix::net::UnixDatagram::pair().unwrap();

		fn make_v6(tail: u16) -> ::core::net::Ipv6Addr {
			::core::net::Ipv6Addr::new(0x2001,0x8003,0x900d,0xd400,0,0,0,tail)
		}
		fn make_v4(tail: u8) -> ::core::net::Ipv4Addr {
			::core::net::Ipv4Addr::new(10, 0, 2,tail)
		}
		const MTU: usize = 1520;
		let slirp_options = ::libslirp::Opt {
			restrict: false,
			mtu: MTU,
			disable_host_loopback: false,
			hostname: None,
			dns_suffixes: Vec::new(),
			domainname: None,
			ipv4: ::libslirp::opt::OptIpv4 {
				disable: false,
				net: ::ipnetwork::Ipv4Network::new(make_v4(0), 24).unwrap(),
				host: make_v4(1),
				dhcp_start: make_v4(16),
				dns: make_v4(2),
			},
			ipv6: ::libslirp::opt::OptIpv6 {
				disable: true,
				net6: ::ipnetwork::Ipv6Network::new(make_v6(0), 64).unwrap(),
				host: make_v6(1),
				dns: make_v6(2),
			},
			tftp: ::libslirp::opt::OptTftp {
				name: None,
				root: None,
				bootfile: None,
			},
		};
		let sock_outer = Arc::new(sock_outer);
		let rv = Nic {
			tx_sender: sock_outer.clone(),
			rx_common: rx_common.clone(),
		};
		::std::thread::spawn(move || {
			use ::std::os::fd::AsRawFd;
			const MIO_TOKEN_SOCK: ::mio::Token = ::mio::Token(9999);
			let poll = ::mio::Poll::new().unwrap();
			let mio_slirp = ::libslirp::MioHandler::new(&slirp_options, &poll, sock_slirp);
			mio_slirp.register();
			poll.register(
				&::mio::unix::EventedFd(&sock_outer.as_raw_fd()),
				MIO_TOKEN_SOCK,
				::mio::Ready::readable(),
				::mio::PollOpt::level()
			).expect("Poll register `sock_outer`");
			let mut events = ::mio::Events::with_capacity(1024);
    		let mut duration = None;
			loop {
				poll.poll(&mut events, duration).expect("MIO Poll");
				duration = mio_slirp.dispatch(&events).expect("SLIRP MIO Dispatch");
				for event in &events {
					if event.token() == MIO_TOKEN_SOCK {
						let mut msg = vec![0; MTU];
						match sock_outer.recv(&mut msg) {
						Ok(len) => {
							msg.truncate(len);
							let _ = rx_common.out_file.lock().unwrap().push_packet(&msg);
							rx_common.packets.push(msg);
							if let Some(w) = rx_common.waiter.take() {
								w.signal();
							}
						},
						Err(e) => eprintln!("Internal socket error: {:?}", e),
						}
					}
				}
			}
		});
		rv
	}
}

impl ::network::nic::Interface for Nic {
	fn tx_raw(&self, pkt: ::network::nic::SparsePacket) {
		let pkt: Vec<_> = pkt.into_iter().flat_map(|v| v.iter().copied()).collect();
		self.rx_common.out_file.lock().unwrap().push_packet(&pkt).unwrap();
		let _ = self.tx_sender.send(&pkt);
	}

	fn rx_wait_register(&self, channel: &::kernel::threads::SleepObject) {
		self.rx_common.waiter.set(channel.get_ref());
	}

	fn rx_wait_unregister(&self, channel: &::kernel::threads::SleepObject) {
		if let Some(v) = self.rx_common.waiter.take() {
			if !v.is_from(channel) {
				self.rx_common.waiter.set(v);
			}
		}
	}

	fn rx_packet(&self) -> Result<::network::nic::PacketHandle, ::network::nic::Error> {
		return match self.rx_common.packets.try_pop() {
			Some(p) => Ok( ::network::nic::PacketHandle::new(P(p)).ok().unwrap() ),
			None => Err(::network::nic::Error::NoPacket),
			};
		struct P(Vec<u8>);
		impl ::network::nic::RxPacket for P {
			fn len(&self) -> usize {
				self.0.len()
			}
		
			fn num_regions(&self) -> usize {
				1
			}
		
			fn get_region(&self, idx: usize) -> &[u8] {
				assert!(idx == 0);
				&self.0
			}
		
			fn get_slice(&self, range: ::core::ops::Range<usize>) -> Option<&[u8]> {
				self.0.get(range)
			}
		}
	}
}
