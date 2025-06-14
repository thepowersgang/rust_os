// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/modules.rs
// - Runtime-initialised modules (handling load order deps)
#[allow(unused_imports)]
use crate::prelude::*;

#[repr(C)]
pub struct ModuleInfo
{
	pub name: &'static str,
	pub init: fn(),
	pub deps: &'static [&'static str],
	pub _rsvd: [usize; 3],
}

#[derive(Clone,PartialEq)]
enum ModuleState
{
	/// Module is not yet initialised
	Uninitialised,
	/// Module is currently initialising, used to detect loops
	Resolving,
	/// Module fully initialised
	Initialised,
	/// Disabled in boot configuration
	Disabled,
	/// Disabled because a dependency is marked as disabled
	DisabledTransitive,
}

#[cfg(feature="test")]
mod _test {
	#![allow(non_upper_case_globals)]
	#[no_mangle]
	static modules_base: () = ();
	#[no_mangle]
	static modules_end: () = ();
}

extern "C" {
	static modules_base: crate::Extern;
	static modules_end: crate::Extern;
}

/// Initialise statically linked modules
///
/// This is the core initialisation method for the kernel, called to initialise
/// the rest of the kernel.
///
/// - `requests` is a list of modules that should be loaded as soon as possible (e.g. the GUI)
/// - `disabled` lists module names that should not be loaded (e.g. if they break)
pub fn init(requests: &[&str])
{
	let (baseptr, size);
	// SAFE: Data behind the static doesn't change
	unsafe {
		baseptr = &modules_base as *const _ as *const ModuleInfo;
		size = &modules_end as *const _ as usize - baseptr as usize;
	}
	let count = size / ::core::mem::size_of::<ModuleInfo>();
	log_debug!("baseptr={:p}, size={:#x}, count={}", baseptr, size, count);
	assert!(count < 1024);
	assert!(count > 0);

	// SAFE: Pointer should be valid (from linker script)
	unsafe {
		let mods = ::core::slice::from_raw_parts(baseptr, count);
		init_modules(mods, requests);
	}
}

/// Initialise modules from a slice
fn init_modules(mods: &[ModuleInfo], requests: &[&str])
{
	log_debug!("s_modules={:p}+{:#x}", mods.as_ptr(), mods.len());
	for m in mods.iter() {
		log_debug!("mod = {:p} {:?} '{}'", &m.name, m.name.as_ptr(), m.name);
	}

	let mut modstates = vec![ModuleState::Uninitialised; mods.len()];
	// Diasble modules requested at boot
	for name in crate::config::disabled_module_names() {
		for (ms, m) in Iterator::zip(modstates.iter_mut(), mods.iter()) {
			if m.name == name {
				*ms = ModuleState::Disabled;
			}
		}
	}
	for req in requests {
		if !init_module_by_name(&mut modstates, mods, "", req) {
			log_error!("Requested module {:?} not able to start due to boot-disabled module", req);
		}
	}
	
	for i in 0 .. mods.len() {
		init_module(&mut modstates, mods, i);
	}
}

/// Initialise a module by name, as required by another module
///
/// `req` = requesting module, `name` = required module
fn init_module_by_name(modstates: &mut [ModuleState], mods: &[ModuleInfo], req: &str, name: &str) -> bool
{
	// Locate module
	let depid = match mods.iter().enumerate().find( |&(_,v)| v.name == name ) {
		Some( (depid,_) ) => depid,
		None => panic!("Dependency '{}' for module '{}' missing", name, req),
		};
	// Check if not being initialised
	if modstates[depid] == ModuleState::Resolving {
		panic!("Circular dependency '{}' requires '{}' which is already being resolved", req, name);
	}
	
	// Initialise
	init_module(modstates, mods, depid)
}

/// Initialise a module (does nothing if the module is already initialised)
fn init_module(modstates: &mut [ModuleState], mods: &[ModuleInfo], i: usize) -> bool
{
	let module = &mods[i];
	if modstates[i] == ModuleState::Uninitialised
	{
		modstates[i] = ModuleState::Resolving;
		log_debug!("#{}: {} Deps", i, module.name);
		for name in module.deps.iter() {
			// If any dependency is disabled, then return `false`
			if !init_module_by_name(modstates, mods, module.name, *name) {
				log_error!("#{}: {} Cannot start, a dependency is disabled (trying {})", i, module.name, name);
				modstates[i] = ModuleState::DisabledTransitive;
				return false;
			}
		}
		// TODO: Do module initialisation in worker threads, and handle waiting for deps before calling init
		log_debug!("#{}: {} Init", i, module.name);
		(module.init)();
		modstates[i] = ModuleState::Initialised;
	}
	
	!matches!(modstates[i], ModuleState::Disabled | ModuleState::DisabledTransitive)
}

// vim: ft=rust

