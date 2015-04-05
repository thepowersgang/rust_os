// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/kernel_log.rs
// - Kernel log output (and debug)

use _common::*;

use super::windows::WindowGroupHandle;
use super::windows::WindowHandle;

struct KernelLog
{
	wgh: WindowGroupHandle,
	wh: WindowHandle,
	cur_line: u32,
}

static S_KERNEL_LOG: ::sync::mutex::LazyMutex<KernelLog> = lazymutex_init!();

pub fn init()
{
	// Create window (and structure)
	let mut wgh = WindowGroupHandle::alloc("Kernel");
	let wh = wgh.create_window();
	S_KERNEL_LOG.init(|| KernelLog {
		wgh: wgh,
		wh: wh,
		cur_line: 0
		});
	// Populate kernel logging window with accumulated logs
	// Register to recieve logs
}

