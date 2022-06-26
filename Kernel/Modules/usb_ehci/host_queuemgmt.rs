//! HostInner async queue management functions
//! 
use ::core::sync::atomic::Ordering;
use crate::desc_pools;
use crate::hw_regs;
use crate::hw_structs;

/// Async queue management
impl super::HostInner
{
    /// Add the specified QH to the async list
    pub(crate) fn add_qh_to_async(&self, mut qh: desc_pools::QhHandle) -> HostHeldQh
    {
        // Add to the async queue
        {
            let mut lh = self.async_head_td.lock();
            let dead_qh_data = self.qh_pool.get_data_mut(&mut lh);
            let this_qh_data = self.qh_pool.get_data_mut(&mut qh);
            this_qh_data.current_td = 1;    // Safety
            this_qh_data.overlay_link = 1;  // Safety
            this_qh_data.overlay_token = hw_structs::QTD_TOKEN_STS_HALT;    // Prevents execution of the queue
            // - `hlink` to point to the dead QH's next
            this_qh_data.hlink = dead_qh_data.hlink;
            // - Set dead QH's next to this
            dead_qh_data.hlink = self.qh_pool.get_phys(&qh) | hw_structs::QH_HLINK_TY_QH;
        }

        HostHeldQh {
            qh
        }
    }

    /// Modify an endpoint's settings
    /// 
    // NOTE: This is sound, as it requires mutable access (thus the endpoint can't be active)
    pub(crate) fn edit_endpoint(&'_ self, qh: &'_ mut HostHeldQh) -> (&'_ mut u32, &'_ mut u32) {
        let qh = self.qh_pool.get_data_mut(&mut qh.qh);
        (&mut qh.endpoint, &mut qh.endpoint_ext)
    }

    /// Remove/release the passed Qh
    pub(crate) fn remove_qh_from_async(&self, qh: HostHeldQh)
    {
        // Note: QH can't be active, as this function now has ownership.

        // SAFE: This queue head is on the list (as we have a `HostHeldQh` instance)
        unsafe {
            let mut lh = self.async_head_td.lock();
            // Remove the QH from the async list (iterate the list and stitch it up)
            self.qh_pool.remove_from_list(&mut lh, &qh.qh);
            // Release the QH from the pool
            self.qh_pool.release(qh.qh);
            // If not running, trigger an immediate GC
            if self.regs.read_op(hw_regs::OpReg::UsbSts) & hw_regs::USBSTS_AsyncEnabled == 0 {
                self.qh_pool.trigger_gc();
            }
            else {
                // if running, set IAAD
                self.regs.write_op(hw_regs::OpReg::UsbCmd, self.regs.read_op(hw_regs::OpReg::UsbCmd) | hw_regs::USBCMD_IAAD);
            }
        }
    }

    /// Start an async transaction
    pub(crate) async fn wait_for_async(&self, qh: &mut HostHeldQh, mut first_td: desc_pools::TdHandle) -> desc_pools::TdHandle
    {
        log_debug!("wait_for_async({:?}): first_td={:?}", qh, first_td);
        // REF: EHCI spec, 4.8 "Asynchronous Schedule"

        // Ensure IOC is set for the final entry in the td chain
        self.td_pool.iter_chain_mut(&mut first_td, |data/*, _meta*/| {
            log_debug!("TD {:#x} {:?}", ::kernel::memory::virt::get_phys(data), data);
            data.token |= hw_structs::QTD_TOKEN_STS_ACTIVE;
            if data.link & 1 != 0 {
                data.token |= hw_structs::QTD_TOKEN_IOC;
            }
            });
        // Add the TD to the queue header
        self.qh_pool.assign_td(&mut qh.qh, &self.td_pool, first_td);

        // Re-start the async queue.
        // - If it's already running, flag for a restart on exhaustion?
        // SAFE: Called with the lock held
        unsafe
        {
            let mut async_head = self.async_head_td.lock();
            // - Flag so it's started next time there's a run through
            self.async_run_request.store(true, Ordering::SeqCst);
            // - If stopped (see USBSTS), then kick it
            self.start_async_queue(&mut async_head);
        }
        // Wait for this queue to complete
        log_debug!("wait_for_async({:?}): Sleeping", qh.qh);
        ::kernel::futures::drop_wrapper(
            self.qh_pool.wait(&mut qh.qh), 
            || { todo!("Cancel EHCI future"); } // TODO: Handle cancellation
            ).await;
        //self.qh_pool.wait(&mut qh.qh).await;
        log_debug!("wait_for_async({:?}): Complete", qh.qh);
        log_debug!("wait_for_async({:?}): QH {:#x} = {:?}", qh.qh, ::kernel::memory::virt::get_phys(self.qh_pool.get_data(&qh.qh)), self.qh_pool.get_data(&qh.qh));
        // Remove the TD handle from the queue
        self.qh_pool.clear_td(&mut qh.qh).unwrap()

        // TODO: Stop the async queue if nothing to do?
    }

    /// Re-start the stopped async queue
    /// 
    /// NOTE: Public so the parent (the interrupt code) can run it
    /// UNSAFE: Can only be called with the async queue lock held
    pub(crate) unsafe fn start_async_queue(&self, async_head: &mut desc_pools::QhHandle)
    {
        if self.regs.read_op(hw_regs::OpReg::UsbSts) & hw_regs::USBSTS_AsyncEnabled == 0 {
            log_debug!("start_async_queue");
            self.async_run_request.store(false, Ordering::SeqCst);
            let next = self.qh_pool.get_data(&async_head).hlink & !0xF;
            self.regs.write_op(hw_regs::OpReg::AsyncListAddr, next);
            self.regs.write_op(hw_regs::OpReg::UsbCmd,
                self.regs.read_op(hw_regs::OpReg::UsbCmd)
                | hw_regs::USBCMD_AsyncEnable   // Start the async queue
                );
        }
    }
}

#[derive(Debug)]
pub struct HostHeldQh
{
    qh: desc_pools::QhHandle
}