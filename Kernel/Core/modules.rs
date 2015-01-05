// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/modules.rs
// - Runtime-initialised modules (handling load order deps)
use _common::*;

#[repr(packed)]
#[allow(missing_copy_implementations)]
pub struct ModuleInfo
{
	pub name: &'static str,
	pub init: fn(),
	pub deps: &'static [&'static str],
	pub _rsvd: uint,
}

#[derive(Clone,PartialEq)]
enum ModuleState
{
	Uninitialised,
	Resolving,
	Initialised,
}

extern "C" {
	static modules_base: ();
	static modules_end: ();
}

pub fn init()
{
	let baseptr = &modules_base as *const _ as *const ModuleInfo;
	let size = &modules_end as *const _ as uint - baseptr as uint;
	let count = size / ::core::mem::size_of::<ModuleInfo>();
	
	unsafe {
		let mods = ::core::slice::from_raw_buf(&baseptr, count);
		init_modules(mods);
	}
}

fn init_modules(mods: &[ModuleInfo])
{
	log_debug!("s_modules={},{:#x}", mods.as_ptr(), mods.len());
	let mut modstates = Vec::from_elem(mods.len(), ModuleState::Uninitialised);
	for m in mods.iter() {
		log_debug!("mod = {} {} '{}'", &m.name as *const _, m.name.as_ptr(), m.name);
	}
	for i in range(0, mods.len())
	{
		init_module(modstates.slice_mut(), mods, i);
	}
}

fn init_module(modstates: &mut [ModuleState], mods: &[ModuleInfo], i: uint)
{
	let module = &mods[i];
	if modstates[i] == ModuleState::Uninitialised
	{
		modstates[i] = ModuleState::Resolving;
		log_debug!("#{}: {}", i, module.name);
		for name in module.deps.iter() {
			// Locate module
			let depid = match mods.iter().enumerate().find( |&(_,v)| v.name==*name ) {
				Some( (depid,_) ) => depid,
				None => panic!("Dependency '{}' for module '{}' missing", *name, module.name),
				};
			// Check if not being initialised
			if modstates[depid] == ModuleState::Resolving {
				panic!("Circular dependency '{}' requires '{}' which is already being resolved", module.name, *name);
			}
			// Initialise
			init_module(modstates, mods, depid);
		}
		(module.init)();
		modstates[i] = ModuleState::Initialised;
	}
}

// vim: ft=rust

