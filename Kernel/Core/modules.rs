//
//
//
use _common::*;

#[repr(packed)]
pub struct ModuleInfo
{
	pub name: &'static str,
	pub init: fn(),
	pub deps: &'static [&'static str],
	pub _rsvd: uint,
}

#[deriving(Clone)]
#[deriving(PartialEq)]
enum ModuleState
{
	ModUninitialised,
	ModResolving,
	ModInitialised,
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
		::core::slice::raw::buf_as_slice(baseptr, count, init_modules);
	}
}

fn init_modules(mods: &[ModuleInfo])
{
	log_debug!("s_modules={},{:#x}", mods.as_ptr(), mods.len());
	let mut modstates = Vec::from_elem(mods.len(), ModUninitialised);
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
	if modstates[i] == ModUninitialised
	{
		modstates[i] = ModResolving;
		log_debug!("#{}: {}", i, module.name);
		for name in module.deps.iter() {
			// Locate module
			let depid = match mods.iter().enumerate().find(|&(i,v)| {log_debug!("v.name={}", v.name); v.name==*name}) {
				Some( (depid,_) ) => depid,
				None => fail!("Dependency '{}' for module '{}' missing", *name, module.name),
				};
			// Check if not being initialised
			if modstates[depid] == ModResolving {
				fail!("Circular dependency '{}' requires '{}' which is already being resolved", module.name, *name);
			}
			// Initialise
			init_module(modstates, mods, depid);
		}
		(module.init)();
		modstates[i] = ModInitialised;
	}
}

// vim: ft=rust

