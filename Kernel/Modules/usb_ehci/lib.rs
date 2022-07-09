// "Tifflin" Kernel - EHCI USB driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_ehci/lib.rs
//! Extensible Host Controller Interface (EHCI) driver
#![no_std]
#![feature(linkage)]	// for module_define!
#![feature(ptr_metadata)]    // for hackery in `UsbHost::set_hub_port_speed`
#[macro_use]
extern crate kernel;

use ::core::sync::atomic::{Ordering,AtomicU32};
use ::kernel::prelude::*;
use ::kernel::lib::mem::aref::Aref;

mod hw_regs;
mod hw_structs;
mod pci;
mod usb_host;
mod desc_pools;

mod host_queuemgmt;
use self::host_queuemgmt::HostHeldQh;
mod host_interrupt;

::kernel::module_define!{usb_ehci, [usb_core], init}

fn init()
{
	static PCI_DRIVER: pci::PciDriver = pci::PciDriver;
	::kernel::device_manager::register_driver(&PCI_DRIVER);
}


type HostRef = ::kernel::lib::mem::aref::ArefBorrow<HostInner>;

struct BusDev
{
    _host: Aref<HostInner>,
}
impl BusDev
{
	fn new_boxed(irq: u32, io: ::kernel::device_manager::IOBinding) -> ::kernel::device_manager::DriverBindResult
	{
		Ok(::kernel::device_manager::DriverInstancePtr::new(BusDev {
			_host: HostInner::new_aref(irq, io)?
			}))
	}
}
impl ::kernel::device_manager::DriverInstance for BusDev
{
}


struct HostInner
{
	_irq_handle: Option<::kernel::irqs::ObjectHandle>,

    regs: hw_regs::Regs,
    /// 
    periodic_queue: ::kernel::sync::Mutex< ::kernel::memory::virt::ArrayHandle<u32> >,
    td_pool: desc_pools::TdPool,
    qh_pool: desc_pools::QhPool,
    async_head_td: ::kernel::sync::Spinlock<desc_pools::QhHandle>,

    //
    async_run_request: ::core::sync::atomic::AtomicBool,

	// - Async support
	waker: ::kernel::sync::Spinlock<core::task::Waker>,
	port_update: AtomicU32,

    // - Split transations/USB1
    usb1: ::kernel::sync::Mutex<Vec<Option<usb_host::Usb1>>>,
}
impl HostInner
{
    fn new_aref(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Aref<Self>, ::kernel::device_manager::DriverBindError> {
        // SAFE: The caller of this function (in the current module) passes the correct IO handle
        let regs = unsafe { hw_regs::Regs::new(io) };
		let revision = regs.hci_version();
        let nports = (regs.hcs_params() & 0xF) as u8;
		log_notice!("Card {:?} version is {:#x} w/ {} ports", regs.get_inner(), revision, nports);

        // Allocate a periodic queue
        // NOTE: We _could_ use a 64-bit address, but there's a restriction that all
        let mut periodic_queue = ::kernel::memory::virt::alloc_dma(32, 1, module_path!())?.into_array::<u32>();
        for v in periodic_queue.iter_mut() {
            *v = 1;
        }

        // Initialise TransferDescriptor pool, and make a placeholder for dead slots
        let td_pool = desc_pools::TdPool::new()?;
        //let dead_td = {
        //    // SAFE: Zero-length
        //    let mut dead_td = unsafe { td_pool.alloc(hw_structs::Pid::Out, &[], None) };
        //    td_pool.get_data_mut(&mut dead_td).token = hw_structs::QTD_TOKEN_STS_HALT;
        //    dead_td
        //    };

        // Initialise QueueHeader pool, and make a placeholder for dead slots
        let qh_pool = desc_pools::QhPool::new()?;
        let mut dead_qh = qh_pool.alloc_raw(hw_structs::QueueHead {
            hlink: 2,
            endpoint: hw_structs::QH_ENDPT_H,
            endpoint_ext: 0,
            current_td: 0,//td_pool.get_phys(&dead_td),
            overlay_link: 0,//td_pool.get_phys(&dead_td),
            overlay_link2: 0,
            overlay_token: hw_structs::QTD_TOKEN_STS_HALT,
            overlay_pages: [0; 5],
            });
        qh_pool.get_data_mut(&mut dead_qh).hlink = qh_pool.get_phys(&dead_qh) | 2;
        
        // SAFE: Register accesses are correct
        unsafe {
            use hw_regs::*;
            // Reset the controller
            regs.write_op(OpReg::UsbCmd, USBCMD_HCReset);
            // Set up interrupts
            // - Interrupt on completion
            // - (Root) Port status change
            // - [DISABLED] Frame rollover (every 1024ms)
            // - Async queue advance (only fires when requested)
            regs.write_op(OpReg::UsbIntr, USBINTR_IOC|USBINTR_PortChange/*|USBINTR_FrameRollover*/|USBINTR_IntrAsyncAdvance);
            // Set addresses
            regs.write_op(OpReg::PeriodicListBase, ::kernel::memory::virt::get_phys(&periodic_queue[0]) as u32);
            regs.write_op(OpReg::AsyncListAddr, qh_pool.get_phys(&dead_qh));
            // Enable controller
            regs.write_op(OpReg::UsbCmd, /*interupt threshold*/ (0x40 << 16) | USBCMD_PeriodicEnable | USBCMD_AsyncEnable | USBCMD_Run);
            // Route all ports to the controller
            regs.write_op(OpReg::ConfigFlag, 1);
        }

        let mut inner_aref = Aref::new(HostInner {
            _irq_handle: None,
            regs,
            periodic_queue: ::kernel::sync::Mutex::new(periodic_queue),
            td_pool,
            qh_pool,

            async_head_td: ::kernel::sync::Spinlock::new(dead_qh),
            async_run_request: Default::default(),

            waker: ::kernel::sync::Spinlock::new(kernel::futures::null_waker()),
            port_update: Default::default(),
            usb1: Default::default(),
            });
        
		// Bind interrupt
		{
			struct RawSend<T: Send>(*const T);
			unsafe impl<T: Send> Send for RawSend<T> {}
			let ret_raw = RawSend(&*inner_aref);
			// SAFE: Pointer _should_ be valid as long as this IRQ binding exists
			Aref::get_mut(&mut inner_aref).unwrap()._irq_handle = Some(::kernel::irqs::bind_object(irq, Box::new(move || unsafe { (*ret_raw.0).handle_irq() } )));
		}
        
		::usb_core::register_host(Box::new(usb_host::UsbHost { host: inner_aref.borrow() }), nports);

        Ok(inner_aref)
    }

    fn nports(&self) -> u8 {
        (self.regs.hcs_params() & 0xF) as u8
    }

    fn handle_irq(&self) -> bool
    {
        let orig_sts = self.regs.read_op(hw_regs::OpReg::UsbSts);
        let mut sts = orig_sts & !0xF000;
        if sts != 0 {
            let mut chk = |bit: u32| { let rv = sts & bit != 0; sts &= !bit; rv };
            if chk(hw_regs::USBINTR_IOC) {
                // Interrupt-on-completion
                log_trace!("handle_irq: IOC");

                // - Run through the async list, and check for completed
                // > Completed means that the `Active` bit in the overlay is clear
                // SAFE: Called with the async lock
                //unsafe {
                //    self.qh_pool.check_completion(&self.async_head_td.lock());
                //}

                // TODO: Run completion on all entries? Needed for interrupt endpoints
                self.qh_pool.check_any_complete();
            }
            // Async queue has advanced (i.e. OpReg::AsyncListAddr has updated)
            if chk(hw_regs::USBINTR_IntrAsyncAdvance) {
                log_trace!("handle_irq: IntrAsyncAdvance");
                let mut async_head_td = self.async_head_td.lock();
                // Inform the QH queue that it can now GC
                // SAFE: Controller has just informed us of an async advance, and the lock is held (so no other mutations can happen)
                unsafe {
                    // TODO: Is it possible that something is added to the GC queue between the interrupt and the above lock being acquired?
                    self.qh_pool.trigger_gc();
                }
                // If there's a need to re-start the queue? (if this is now stopped, restart)
                if self.async_run_request.load(Ordering::SeqCst) {
                    // SAFE: Lock is held
                    unsafe {
                        self.start_async_queue(&mut async_head_td);
                    }
                }
            }
            // Port change, determine what port and poke helper thread
            if chk(hw_regs::USBINTR_PortChange) {
                for i in 0 .. self.nports() {
                    let sts = self.regs.read_port_sc(i);

                    if sts & (hw_regs::PORTSC_ConnectStatusChange|hw_regs::PORTSC_PortEnableChange|hw_regs::PORTSC_OvercurrentChange) != 0 {
                        // Over-current detected on the port? (well, a change in it)
                        self.port_update.fetch_or(1 << i, Ordering::SeqCst);
                    }
                }
                let pu = self.port_update.load(Ordering::SeqCst);
                log_debug!("handle_irq: PortChange {pu:#x}");
				if pu != 0 {
					self.waker.lock().wake_by_ref();
				}
            }
            if chk(hw_regs::USBINTR_FrameRollover) {
                // Frame rollover, used to aid timing (trigger per-second operations)
            }
            if sts != 0 {
                log_warning!("Unexpected/unhandled interrupt bits");
            }
            // SAFE: Writing to this register does nothing but ACK the interrupt
            unsafe { self.regs.write_op(hw_regs::OpReg::UsbSts, orig_sts) };
            true
        }
        else {
            false
        }
    }
}

/// Handling for USB1.0 devices (split transactions)
impl HostInner
{
    fn set_usb1(&self, dev_id: u8, usb1: Option<usb_host::Usb1>) {
        let mut lh = self.usb1.lock();
        while dev_id as usize >= lh.len() {
            lh.push(None);
        }
        lh[dev_id as usize] = usb1;
    }
    fn get_usb1(&self, dev_id: u8) -> Option<usb_host::Usb1> {
        match self.usb1.lock().get(dev_id as usize)
        {
        Some(&v) => v,
        None => None,
        }
    }
}