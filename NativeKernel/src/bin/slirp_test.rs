fn main() {
	let timers = ::std::sync::Arc::new(Timers {
		base_time: ::std::time::Instant::now(),
		timers: ::std::sync::Mutex::new(Vec::new()),
	});

	let options = ::libslirp::Opt {
		restrict: false,
		mtu: 1520,
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
	let slirp = ::libslirp::Context::new_with_opt(&options, SlirpHandler { timers });
	slirp.input(&[
		// Ethernet
		0x52, 0x55, 0x0a, 0x0, 0x2, 0x1,
		  0xaa, 0xbb, 0xcc, 0x0, 0x0, 0x1,
		  0x8, 0x0,
		// IP
		0x45, 0x0, 0x0, 0x28,
		  0x00, 0x00, 0x00, 0x00,
		  0xff, 0x06, 0xed, 0xf0,
		  0x0a, 0x00, 0x02, 0x10,	// src
		  0xc0, 0xa8, 0x01, 0x27,	// dst
		// TCP
		0xc0, 0x01, 0x1a, 0x0b,	// src, dst
		  0x0, 0x1, 0x0, 0x0,
		  0x0, 0x0, 0x0, 0x0,
		  0x50, 0x2, 0x40, 0x0,
		  0xc7, 0xf5, 0x00, 0x0
		]);
	// 52, 55,  a, 0, 2, 1
	// aa, bb, cc, 0, 0, 1,
	// 8, 0,
	// 45, 0, 0, 28, 0, 0, 0, 0, ff, 6, ed, f0, a, 0, 2, 10, c0, a8, 1, 27, 
	// 0, 1, 1a, b, 0, 1, 0, 0, 0, 0, 0, 0,
	// 50, 2, 40, 0, c7, f5, 0, 0
	println!("{}", slirp.connection_info());
	let mut timeout = !0;
	slirp.pollfds_fill(&mut timeout, |a,_b| a);
	slirp.pollfds_poll(false, |_a| todo!());
	println!("timeout = {}", timeout);
}
fn make_v6(tail: u16) -> ::core::net::Ipv6Addr {
	::core::net::Ipv6Addr::new(0x2001,0x8003,0x900d,0xd400,0,0,0,tail)
}
fn make_v4(tail: u8) -> ::core::net::Ipv4Addr {
	::core::net::Ipv4Addr::new(10, 0, 2,tail)
}
struct SlirpHandler {
	//rx_common: ::std::sync::Arc<RxCommon>,
	timers: ::std::sync::Arc<Timers>,
}
impl ::libslirp::Handler for SlirpHandler {
	type Timer = TimerHandle;

	fn clock_get_ns(&mut self) -> i64 {
		self.timers.cur_time_ns()
	}

	fn send_packet(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		println!("REPLY: {:?}", buf);
		Ok(buf.len())
	}

	fn register_poll_fd(&mut self, _fd: std::os::unix::prelude::RawFd) {
		// ?
		println!("register_poll_fd")
	}

	fn unregister_poll_fd(&mut self, _fd: std::os::unix::prelude::RawFd) {
		// ?
		println!("unregister_poll_fd")
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