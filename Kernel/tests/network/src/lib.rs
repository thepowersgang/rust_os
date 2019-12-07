/*!
 */
#[macro_use]
extern crate kernel;
use std::time::Duration;

const REMOTE_MAC: [u8; 6] = *b"RSK\x12\x34\x56";
const LOCAL_MAC: [u8; 6] = *b"RSK\xFE\xFE\xFE";

mod tcp;
mod ipv4;

#[derive(serde_derive::Deserialize,serde_derive::Serialize)]
struct EthernetHeader
{
    dst: [u8; 6],
    src: [u8; 6],
    proto: u16,
}
impl EthernetHeader
{
    fn encode(&self) -> [u8; 6+6+2] {
        let mut rv = [0; 14];
        {
            let mut c = std::io::Cursor::new(&mut rv[..]);
            bincode::config().big_endian().serialize_into(&mut c, self).unwrap();
            assert!(c.position() == 14);
        }
        rv
    }
}

pub struct TestFramework {
    nic: network::nic::Registration<TestNic>,
}
impl TestFramework
{
    pub fn new() -> TestFramework
    {
        ensure_setup();    
        TestFramework {
            nic: network::nic::register(REMOTE_MAC, TestNic::new()),
        }
    }

    /// Encode+send an ethernet frame to the virtualised NIC (addressed correctly)
    pub fn send_ethernet_direct(&self, proto: u16, buffers: &[ &[u8] ])
    {
        let ethernet_hdr = EthernetHeader { dst: REMOTE_MAC, src: LOCAL_MAC, proto: proto, }.encode();
        let buf: Vec<u8> = Iterator::chain([&ethernet_hdr as &[u8]].iter(), buffers.iter())
            .flat_map(|v| v.iter())
            .copied()
            .collect()
            ;
        self.nic.send_packet(buf);
    }

    fn wait_packet(&self, timeout: Duration) -> Option<Vec<u8>>
    {
        self.nic.wait_packet(timeout)
    }
}

pub struct TestNic
{
    rx: std::sync::Mutex<TestNicRx>,
    tx_sender: std::sync::Mutex<std::sync::mpsc::Sender< Vec<u8> > >,
    tx_receiver: std::sync::Mutex<std::sync::mpsc::Receiver< Vec<u8> > >,
}
#[derive(Default)]
pub struct TestNicRx
{
    waiter_handle: Option<kernel::threads::SleepObjectRef>,
    queue: std::collections::VecDeque< Vec<u8> >,
}
impl TestNic
{
    fn new() -> Self
    {
        let (tx_sender, tx_receiver) = std::sync::mpsc::channel();
        TestNic {
            rx: Default::default(),
            tx_sender: std::sync::Mutex::new(tx_sender),
            tx_receiver: std::sync::Mutex::new(tx_receiver),
            }
    }

    fn send_packet(&self, buf: Vec<u8>)
    {
        let mut lh = self.rx.lock().expect("Poisoned");
        lh.queue.push_back( buf );
        lh.waiter_handle.as_ref().expect("No connected waiter").signal();
    }
    fn wait_packet(&self, timeout: Duration) -> Option<Vec<u8>>
    {
        kernel::threads::yield_time();
        let /*mut*/ lh = self.tx_receiver.lock().expect("TX poisoned");
        match lh.recv_timeout(timeout)
        {
        Ok(v) => Some(v),
        Err(_e) => {
            log_debug!("TestNic::wait_packet: err {:?}", _e);
            None
            },
        }
    }
}
impl network::nic::Interface for TestNic
{
    fn tx_raw(&self, pkt: network::nic::SparsePacket<'_>) {
        let buf: Vec<u8> = pkt.into_iter().flat_map(|v| v.iter()).copied().collect();
        self.tx_sender.lock().unwrap().send(buf);
    }
    fn tx_async<'a,'s>(&'s self, _: kernel::_async3::ObjectHandle, _: kernel::_async3::StackPush<'a, 's>, _: network::nic::SparsePacket<'_>) -> Result<(), network::nic::Error> {
        todo!("TestNic::tx_async")
    }
    fn rx_wait_register(&self, channel: &kernel::threads::SleepObject<'_>) {
        self.rx.lock().unwrap().waiter_handle = Some(channel.get_ref());
    }
	fn rx_wait_unregister(&self, channel: &kernel::threads::SleepObject) {
        self.rx.lock().unwrap().waiter_handle = None;
    }

    fn rx_packet(&self) -> Result<network::nic::PacketHandle, network::nic::Error> {
        let mut lh = self.rx.lock().expect("RX poisoned");
        if let Some(v) = lh.queue.pop_front()
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

fn ensure_setup()
{
    use std::sync::atomic::{Ordering, AtomicUsize};
    static STARTED: AtomicUsize = AtomicUsize::new(0);
    if STARTED.compare_and_swap(0, 1, Ordering::SeqCst) == 0 {
        kernel::threads::init();
        (network::S_MODULE.init)();
        STARTED.store(2, Ordering::SeqCst);
    }
    else {
        while STARTED.load(Ordering::SeqCst) == 1 {
            // Spin!
        }
    }
}
