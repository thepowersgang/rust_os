//! 
//! 
//! 
use ::kernel_test_network::{HexDump,MessageStream};

pub struct TestNicHandle
{
    number: u32,
    stream: MessageStream,
    mac: [u8; 6],
    netif: ::std::cell::UnsafeCell<::lwip::sys::netif>,
}
impl TestNicHandle
{
    pub(super) fn new(number: u32, stream: MessageStream, mac: [u8; 6], ip: ::lwip::sys::ip4_addr_t, mask_bits: u8) -> &'static TestNicHandle {
        let rv = Box::new(TestNicHandle {
            number,
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
            //(*netif_ptr).state = state_ptr;
            let b = ::std::sync::Arc::new(::std::sync::Barrier::new(2));
            let b2 = b.clone();
            ::lwip::os_mode::callback(move || {
                let rv = ::lwip::sys::netif_add(
                    netif_ptr, &ip, &mask, &gw,
                    state_ptr, Some(Self::init), Some(::lwip::sys::tcpip_input)
                    );
                assert!(!rv.is_null(), "netif_add failed");
                assert!(rv == netif_ptr, "netif_add failed");
                b2.wait();
            });
            b.wait();
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

    /// Called by the test RX thread to pass a packet into the code-under-test
    pub fn packet_received(&self, buf: Vec<u8>) {
		println!("RX #{} {:?}", self.number, HexDump(&buf));
        unsafe {
            let pbuf =  ::lwip::sys::pbuf_alloc(::lwip::sys::pbuf_layer_PBUF_RAW, buf.len() as u16, ::lwip::sys::pbuf_type_PBUF_RAM);
            ::core::ptr::copy_nonoverlapping(buf.as_ptr(), (*pbuf).payload as *mut _, buf.len());
            let input_fcn = (&*self.netif.get()).input.expect("netif.input is null?");
            input_fcn(pbuf, self.netif.get());
        }
    }

    unsafe extern "C" fn etharp_output(netif: *mut ::lwip::sys::netif, pbuf: *mut ::lwip::sys::pbuf, ipaddr: *const ::lwip::sys::ip4_addr_t) -> ::lwip::sys::err_t {
        ::lwip::sys::etharp_output(netif, pbuf, ipaddr)
    }

    unsafe extern "C" fn linkoutput(this_r: *mut ::lwip::sys::netif, pbuf: *mut ::lwip::sys::pbuf) -> ::lwip::sys::err_t {
        let this = &*((*this_r).state as *const TestNicHandle);
        
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

		println!("TX #{} {:?}", this.number, HexDump(&buf[4..]));
        this.stream.send(this.number, &buf).unwrap();

        ::lwip::sys::err_enum_t_ERR_OK as i8
    }
}