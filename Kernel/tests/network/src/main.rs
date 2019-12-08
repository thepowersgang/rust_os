/*!
 * Network stack wrapper
 */
struct Args
{
	master_ip: std::net::IpAddr,

	sim_ip: network::ipv4::Address,
}

fn main()
{
    kernel::threads::init();
	(network::S_MODULE.init)();

	let args = {
		let mut it = std::env::args();
		Args {
			master_ip: it.next().unwrap().parse().unwrap(),
			sim_ip: {
				let std_ip: std::net::Ipv4Addr = it.next().unwrap().parse().unwrap();
				let o = std_ip.octets();
				network::ipv4::Address::new(o[0], o[1], o[2], o[3])
				},
			}
		};

    let stream = match std::net::UdpSocket::bind( (args.master_ip, 1234) )
        {
        Ok(v) => v,
        Err(e) => {
            println!("Cannot connect to server: {}", e);
            return
            },
        };
    let mac = *b"RSK\x12\x34\x56";
    let nic_handle = network::nic::register(mac, TestNic::new(stream));

    network::ipv4::add_interface(mac, args.sim_ip/*, 24*/);

    loop
    {
		const MTU: usize = 1560;

		let mut buf = [0; MTU];

		match nic_handle.stream.recv(&mut buf)
		{
		Ok(len) => {
			let buf = buf[..len].to_owned();
			nic_handle.packets.lock().unwrap().push_back( buf );
			match *nic_handle.waiter.lock().unwrap()
			{
			Some(ref v) => v.signal(),
			None => println!("No registered waiter yet?"),
			}
			},
		Err(e) => {
			println!("Error reading: {:?}", e);
			break;
			},
		}
    }
}

struct TestNic
{
    stream: std::net::UdpSocket,
    waiter: std::sync::Mutex< Option<kernel::threads::SleepObjectRef> >,
    // NOTE: Kernel sync queue
    packets: std::sync::Mutex< std::collections::VecDeque< Vec<u8> > >,
}

impl TestNic
{
    fn new(stream: std::net::UdpSocket) -> Self
    {
        TestNic {
            stream,
            waiter: Default::default(),
            packets: Default::default(),
            }
    }
}
impl network::nic::Interface for TestNic
{
    fn tx_raw(&self, _: network::nic::SparsePacket<'_>) {
        todo!("TestNic::tx_raw")
    }
    fn tx_async<'a,'s>(&'s self, _: kernel::_async3::ObjectHandle, _: kernel::_async3::StackPush<'a, 's>, _: network::nic::SparsePacket<'_>) -> Result<(), network::nic::Error> {
        todo!("TestNic::tx_async")
    }
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
