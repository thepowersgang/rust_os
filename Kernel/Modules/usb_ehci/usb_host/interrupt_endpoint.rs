//!
use ::usb_core::host::{self,EndpointAddr};
use crate::hw_structs;

pub struct InterruptEndpoint
{
    host: crate::HostRef,
    endpoint: EndpointAddr,
    ih: Option<::kernel::futures::Mutex<crate::host_interrupt::IntHandle>>,   // Option so it can be removed
    buf: ::kernel::lib::Vec<u8>,
    next_td: ::kernel::sync::Spinlock<(bool, Option<crate::desc_pools::TdHandle>,)>,
}

impl InterruptEndpoint
{
    pub(super) fn new(host: crate::HostRef, endpoint: EndpointAddr, period_ms: usize, max_packet_size: usize) -> Self {
        let usb1 = host.get_usb1(endpoint.dev_addr());
        let (endpoint_id, endpoint_ext) = super::make_endpoint_spec(endpoint, max_packet_size, usb1, false);
        let qh = host.qh_pool.alloc(endpoint_id, endpoint_ext);

        let buf = vec![0; max_packet_size * 2];
        // SAFE: The buffer here is held for longer than the QH (and thus the TDs) lives
        let (td1,td2) = unsafe { (
            host.td_pool.alloc(hw_structs::Pid::In, &buf[..max_packet_size], None),
            host.td_pool.alloc(hw_structs::Pid::In, &buf[max_packet_size..], None),
            )};
        log_debug!("InterruptEndpoint::new: {:?} {} ms {} b - {:?} TDs={:#x} {:#x}",
            endpoint, period_ms, max_packet_size,
            qh,
            host.td_pool.get_phys(&td1),
            host.td_pool.get_phys(&td2),
            );
        let ih = host.add_qh_to_interrupt(qh, period_ms, td1);
        Self {
            host,
            endpoint,
            ih: Some(::kernel::futures::Mutex::new(ih)),
            buf,
            next_td: ::kernel::sync::Spinlock::new((false, Some(td2),) ),
        }
    }
}

impl host::InterruptEndpoint for InterruptEndpoint
{
	fn wait<'a>(&'a self) -> host::AsyncWaitIo<'a, host::IntBuffer<'a>>
    {
        let cap = self.buf.len() / 2;

        log_trace!("InterruptEndpoint::wait({:?})", self.endpoint);
        let (is_second, mut next) = {
            let mut lh = self.next_td.lock();
            let is_second = lh.0;
            lh.0 = !is_second;
            (is_second, lh.1.take().expect("BUG: `InterruptEndpoint::wait` called before previous dropped"))
            };
        // SAFE: The length is kept correct
        unsafe
        {
            let mut td = self.host.td_pool.get_data_mut(&mut next);
            td.token = (td.token & 0x8000FFFF) | ((cap as u32) << 16);
        }
        
        super::make_asyncwaitio(async move {
            let mut s = self.ih.as_ref().unwrap().async_lock().await;
            let td = self.host.wait_for_interrupt(&mut s, next).await;

            match host::IntBuffer::new(IntBuffer {
                parent: self,
                td: Some(td),
                is_second,
                })
            {
            Ok(v) => v,
            Err(_) => panic!("IntBuffer doesn't fit in `Handle` - req {} got {}",
                ::core::mem::size_of::<IntBuffer>(),
                ::core::mem::size_of::<host::IntBuffer>() - ::core::mem::size_of::<usize>(),
                ),
            }
        })
	}
}

struct IntBuffer<'a>
{
    parent: &'a InterruptEndpoint,
    td: Option<crate::desc_pools::TdHandle>,
    is_second: bool,
}
impl<'a> ::usb_core::handle::RemoteBuffer for IntBuffer<'a>
{
    fn get(&self) -> &[u8] {
        let cap = self.parent.buf.len() / 2;
        let remain = hw_structs::TransferDesc::token_len( self.parent.host.td_pool.get_data(self.td.as_ref().unwrap()).token );
        assert!(remain <= cap);
        let len = cap - remain;
        &self.parent.buf[cap * self.is_second as usize .. ][.. len]
    }
}
impl<'a> ::core::ops::Drop for IntBuffer<'a>
{
    fn drop(&mut self)
    {
        // If the next is already populated, then need to re-assign back into the ih
        let mut lh = self.parent.next_td.lock();
        assert!(lh.1.is_none());
        lh.1 = self.td.take();
    }
}