//! Network simulaed using SLIRP (same library as used by qemu's user networking)

pub struct Nic {
	tx_sender: ::std::sync::mpsc::Sender<Vec<u8>>,
	rx_common: ::std::sync::Arc<RxCommon>,
}
unsafe impl Send for Nic { }
unsafe impl Sync for Nic { }
struct SlirpHandler {
	rx_common: ::std::sync::Arc<RxCommon>,
	timers: ::std::sync::Arc<Timers>,
}
impl ::libslirp::Handler for SlirpHandler {
	type Timer = TimerHandle;

	fn clock_get_ns(&mut self) -> i64 {
		self.timers.cur_time_ns()
	}

	fn send_packet(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		::kernel::log_debug!("SLIRP Emitted: {:x?}", buf);
		self.rx_common.out_file.lock().unwrap().push_packet(buf).unwrap();
		self.rx_common.packets.push(buf.to_owned());
		if let Some(v) = self.rx_common.waiter.take() {
			v.signal();
			self.rx_common.waiter.set(v);
		}
		Ok(buf.len())
	}

	fn register_poll_fd(&mut self, _fd: std::os::unix::prelude::RawFd) {
		// ?
	}

	fn unregister_poll_fd(&mut self, _fd: std::os::unix::prelude::RawFd) {
		// ?
	}

	fn guest_error(&mut self, msg: &str) {
		panic!("SLIRP Guest error!: {}", msg);
	}

	fn notify(&mut self) {
		todo!("SLIRP notify")
	}

	fn timer_new(&mut self, func: Box<dyn FnMut()>) -> Box<Self::Timer> {
		Box::new(self.timers.alloc(func))
	}

	fn timer_mod(&mut self, timer: &mut Box<Self::Timer>, expire_time_ms: i64) {
		self.timers.update(&**timer, expire_time_ms)
	}

	fn timer_free(&mut self, timer: Box<Self::Timer>) {
		self.timers.remove(*timer)
	}
}
struct Timers {
	base_time: ::std::time::Instant,
	timers: ::std::sync::Mutex< Vec<Option<Timer>> >,
}
struct Timer {
	fire_time_ns: Option<i64>,
	cb: Box<dyn FnMut()>,
}
impl Timers {
	fn cur_time_ns(&self) -> i64 {
		(::std::time::Instant::now() - self.base_time).as_nanos() as i64
	}
	fn alloc(&self, func: Box<dyn FnMut()>) -> TimerHandle {
		let mut lh = self.timers.lock().unwrap();
		if let Some((i,s)) = lh.iter_mut().enumerate().find(|(_,s)| s.is_none()) {
			*s = Some(Timer { fire_time_ns: None, cb: func });
			TimerHandle { index: i }
		}
		else {
			let i = lh.len();
			lh.push(Some(Timer { fire_time_ns: None, cb: func }));
			TimerHandle { index: i }
		}
	}
	fn update(&self, handle: &TimerHandle, expire_time_ms: i64) {
		self.timers.lock().unwrap()[handle.index].as_mut().unwrap().fire_time_ns = Some(expire_time_ms * 1_000_000);
	}
	fn remove(&self, handle: TimerHandle) {
		self.timers.lock().unwrap()[handle.index] = None;
	}
}
struct TimerHandle {
	index: usize,
}

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
		let (tx_send, tx_recv) = ::std::sync::mpsc::channel();

		fn make_v6(tail: u16) -> ::core::net::Ipv6Addr {
			::core::net::Ipv6Addr::new(0x2001,0x8003,0x900d,0xd400,0,0,0,tail)
		}
		fn make_v4(tail: u8) -> ::core::net::Ipv4Addr {
			::core::net::Ipv4Addr::new(10, 0, 2,tail)
		}
		let rv = Nic {
			tx_sender: tx_send,
			rx_common: rx_common.clone(),
		};
		::std::thread::spawn(move || {
			let timers = ::std::sync::Arc::new(Timers {
				base_time: ::std::time::Instant::now(),
				timers: ::std::sync::Mutex::new(Vec::new()),
			});
			let slirp = ::libslirp::Context::new(
				false,	// `restricted` means that the guest can't access the internet (no router)
				true,
				make_v4(0), ::core::net::Ipv4Addr::new(255, 255, 255, 0),
				make_v4(1),
				true,
				make_v6(0), 64,
				make_v6(1),
				None,
				None, None, None,
				make_v4(16),
				make_v4(2),
				make_v6(2),
				vec![],
				None,
				SlirpHandler {
					rx_common,
					timers: timers.clone(),
				});

			loop {
				while let Ok(p) = tx_recv.try_recv() {
					::kernel::log_debug!("SLIRP Input: {:x?}", p);
					slirp.input(&p);
				}
				let empty_fn = Box::new(||{});
				if let Some((i, mut fcn)) = {
					let mut timer_list = timers.timers.lock().unwrap();
					timer_list.iter_mut().enumerate().find_map(|(i,v)| {
						if let Some(t) = v {
							if let Some(time) = t.fire_time_ns {
								if time < timers.cur_time_ns() {
									t.fire_time_ns = None;
									return Some((i, ::std::mem::replace(&mut t.cb, empty_fn.clone())));
								}
							}
						}
						None
					})
				} {
					::kernel::log_debug!("SLIRP Timer #{}", i);
					fcn();
					if let Some(ref mut v) = timers.timers.lock().unwrap()[i] {
						if ::core::ptr::eq(&*v.cb, &*empty_fn) {
							v.cb = fcn;
						}
					}
				}
				::std::thread::sleep(::std::time::Duration::from_millis(100));
			}
		});
		rv
	}
}
impl ::network::nic::Interface for Nic {
	fn tx_raw(&self, pkt: ::network::nic::SparsePacket) {
		let pkt: Vec<_> = pkt.into_iter().flat_map(|v| v.iter().copied()).collect();
		self.rx_common.out_file.lock().unwrap().push_packet(&pkt).unwrap();
		let _ = self.tx_sender.send(pkt);
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
