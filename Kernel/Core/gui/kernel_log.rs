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

static S_KERNEL_LOG: LazyMutex<KernelLog> = lazymutex_init!();

pub fn init()
{
	
}

