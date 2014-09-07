// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap.rs
// - Dynamic memory manager

use core::option::{Option,None,Some};

// --------------------------------------------------------
// Types
enum HeapId
{
	LocalHeap,	// Inaccessible outside of process
	GlobalHeap,	// Global allocations
}

// --------------------------------------------------------
// Globals
//#[link_section(process_local)] static s_local_lock : ::sync::Mutex;
//static s_global_lock : ::sync::Mutex;

// --------------------------------------------------------
// Code
pub fn init()
{
}

pub unsafe fn allocate(heap: HeapId, size: uint) -> Option<*mut ()>
{
	None
}

pub unsafe fn expand(pointer: *mut (), newsize: uint) -> Option<*mut ()>
{
	None
}

pub unsafe fn deallocate(pointer: *mut ())
{
	
}

// vim: ft=rust
