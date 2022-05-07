//
//
//
use ::kernel_test_network::HexDump;
use ::std::sync::Arc;

pub type IpAddr = ::network::ipv4::Address;

pub fn ip_from_std(std_ip: &::std::net::Ipv4Addr) -> IpAddr {
    let o = std_ip.octets();
    network::ipv4::Address::new(o[0], o[1], o[2], o[3])
}

pub fn init() {
    ::kernel::threads::init();
    (::network::S_MODULE.init)();
}

pub fn create_interface(stream: Arc<::std::net::UdpSocket>, number: u32, mac: [u8; 6], addr: IpAddr) -> &'static mut ::network::nic::Registration<TestNic> {
    let nic_handle = network::nic::register(mac, TestNic::new(number, stream));
	// TODO: Make this a command instead
    network::ipv4::add_interface(mac, addr, 24);
    Box::leak( Box::new(nic_handle) )
}

pub fn spawn_thread(f: impl FnOnce() + Send + 'static) {
    let h = ::kernel::threads::WorkerThread::new("Worker", f);
    ::core::mem::forget(h);
}
pub fn run_blocking<T>(f: impl FnOnce()->T) -> T {
    ::kernel::arch::imp::threads::test_pause_thread(f)
}


pub struct TestNic
{
	number: u32,
    stream: Arc<std::net::UdpSocket>,
    waiter: std::sync::Mutex< Option<kernel::threads::SleepObjectRef> >,
    // NOTE: Kernel sync queue
    packets: std::sync::Mutex< std::collections::VecDeque< Vec<u8> > >,
}

impl TestNic
{
    fn new(number: u32, stream: Arc<std::net::UdpSocket>) -> TestNic
    {
        TestNic {
			number,
            stream,
            waiter: Default::default(),
            packets: Default::default(),
            }
    }

	pub fn packet_received(&self, buf: Vec<u8>)
	{	
        log_notice!("RX #{} {:?}", self.number, HexDump(&buf));
		self.packets.lock().unwrap().push_back( buf );
		match *self.waiter.lock().unwrap()
		{
		Some(ref v) => v.signal(),
		None => println!("No registered waiter yet?"),
		}
	}
}
impl network::nic::Interface for TestNic
{
    fn tx_raw(&self, pkt: network::nic::SparsePacket<'_>) {
		let it = pkt.into_iter().flat_map(|v| v.iter());
		let num_enc = self.number.to_le_bytes();
		let it = Iterator::chain( num_enc.iter(), it );
        let buf: Vec<u8> = it.copied().collect();
		log_notice!("TX #{} {:?}", self.number, HexDump(&buf));
        self.stream.send(&buf).unwrap();
    }
    //fn tx_async<'a,'s>(&'s self, _: kernel::_async3::ObjectHandle, _: kernel::_async3::StackPush<'a, 's>, _: network::nic::SparsePacket<'_>) -> Result<(), network::nic::Error> {
    //    todo!("TestNic::tx_async")
    //}
    fn rx_wait_register(&self, channel: &kernel::threads::SleepObject<'_>) {
        *self.waiter.lock().unwrap() = Some(channel.get_ref());
    }
	fn rx_wait_unregister(&self, _channel: &kernel::threads::SleepObject) {
        *self.waiter.lock().unwrap() = None;
    }

    fn rx_packet(&self) -> Result<network::nic::PacketHandle, network::nic::Error> {
        let mut lh = self.packets.lock().unwrap();
        if let Some(v) = lh.pop_front()
        {
            struct RxPacketHandle(Vec<u8>);
            impl<'a> network::nic::RxPacket for RxPacketHandle {
                fn len(&self) -> usize {
                    self.0 .len()
                }
                fn num_regions(&self) -> usize {
                    1
                }
                fn get_region(&self, idx: usize) -> &[u8] {
                    assert!(idx == 0);
                    &self.0
                }
                fn get_slice(&self, range: ::core::ops::Range<usize>) -> Option<&[u8]> {
                    let b = self.get_region(0);
                    b.get(range)
                }
            }

            Ok(network::nic::PacketHandle::new(RxPacketHandle(v)).ok().unwrap())
        }
        else
        {
            Err(network::nic::Error::NoPacket)
        }
    }
}