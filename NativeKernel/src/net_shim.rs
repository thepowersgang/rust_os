//! Network simulaed using LWIP 

pub struct Nic {
    netif: *mut ::lwip::sys::netif,
	rx_common: ::std::sync::Arc<RxCommon>,
}
unsafe impl Send for Nic { }
unsafe impl Sync for Nic { }

#[derive(Default)]
struct RxCommon {
	waiter: ::kernel::threads::AtomicSleepObjectRef,
	packets: ::kernel::sync::Queue< Vec<u8> >,
}

impl Nic {
	pub fn new(mac: [u8; 6]) -> Self {
        let ip = ::lwip::sys::ip4_addr_t { addr: u32::from_le_bytes([192,168,1,1]) };
		let mask_bits = 24;
        let mask = ::lwip::sys::ip4_addr_t { addr: (!0u32 << (32 - mask_bits)).swap_bytes() };
        let gw = ::lwip::sys::ip4_addr_t { addr: u32::from_le_bytes([192,168,1,2]) };
        println!("TestNicHandle: {} {} gw {}", ip, mask, gw);
		let (rx_send, rx_recv) = ::std::sync::mpsc::channel();
		let netif_ptr;
        unsafe {
			netif_ptr = Box::into_raw(Box::new(::core::mem::zeroed()));
            let state_ptr = Box::into_raw(Box::new(RxLwip {
				packets: rx_send,
				mac,
			})) as *mut ::std::ffi::c_void;
            ::lwip::os_mode::callback(move || {
                let rv = ::lwip::sys::netif_add(
                    netif_ptr, &ip, &mask, &gw,
                    state_ptr, Some(RxLwip::init), Some(::lwip::sys::tcpip_input)
                    );
				let _ = rv;
            })
        }

		let rx_common = ::std::sync::Arc::new(RxCommon::default());
		let rv = Nic {
			netif: netif_ptr,
			rx_common: rx_common.clone(),
		};

		// Spawn a worker to handle sending packets into the kernel
		::kernel::threads::WorkerThread::new("NIC Input", move || {
			loop {
				let pkt = ::kernel::arch::imp::threads::test_pause_thread(|| rx_recv.recv().expect("Input sender dropped") );
				rx_common.packets.push(pkt);
				if let Some(v) = rx_common.waiter.take() {
					v.signal();
					rx_common.waiter.set(v);
				}
			}
			});
		rv
	}
}
impl ::network::nic::Interface for Nic {
	fn tx_raw(&self, pkt: ::network::nic::SparsePacket) {
		let len = pkt.total_len();
        let pbuf = unsafe {
			::lwip::sys::pbuf_alloc(::lwip::sys::pbuf_layer_PBUF_RAW_TX, len as u16, ::lwip::sys::pbuf_type_PBUF_RAM)
		};
		unsafe {
			let mut dst = (*pbuf).payload as *mut _;
			for buf in pkt.into_iter() {
				::core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, buf.len());
				dst = dst.offset(buf.len() as _);
			}
		}
        let input_fcn = unsafe { (&*self.netif).input.unwrap() };
        unsafe { input_fcn(pbuf, self.netif); }
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
		let Some(p) = self.rx_common.packets.try_pop() else {
			return Err(::network::nic::Error::NoPacket);
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
		Ok( ::network::nic::PacketHandle::new(P(p)).ok().unwrap() )
	}
}

struct RxLwip {
	mac: [u8; 6],
	packets: ::std::sync::mpsc::Sender< Vec<u8> >,
}
impl RxLwip {
    unsafe extern "C" fn init(netif_r: *mut ::lwip::sys::netif) -> ::lwip::sys::err_t {
        let netif = &mut *netif_r;
        let this = &*(netif.state as *const RxLwip);
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

    unsafe extern "C" fn etharp_output(netif: *mut ::lwip::sys::netif, pbuf: *mut ::lwip::sys::pbuf, ipaddr: *const ::lwip::sys::ip4_addr_t) -> ::lwip::sys::err_t {
        ::lwip::sys::etharp_output(netif, pbuf, ipaddr)
    }

    unsafe extern "C" fn linkoutput(this_r: *mut ::lwip::sys::netif, pbuf: *mut ::lwip::sys::pbuf) -> ::lwip::sys::err_t {
        let this = &*((*this_r).state as *const RxLwip);
        
        let buf = {
            let mut buf = Vec::new();
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

		let _ = this.packets.send(buf);

        ::lwip::sys::err_enum_t_ERR_OK as i8
    }
}