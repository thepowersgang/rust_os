//
//
//
use _common::*;

pub struct ModuleInfo
{
	pub name: &'static str,
	pub init: fn()
}

extern "C" {
	#[allow(ctypes)]
	static modules_base: ModuleInfo;
	#[allow(ctypes)]
	static modules_end: ModuleInfo;
}

pub fn init()
{
	let baseptr = &modules_base as *const _;
	let size = &modules_end as *const _ as uint - baseptr as uint;
	let count = size / ::core::mem::size_of::<ModuleInfo>();
	let mods: &[ModuleInfo] = unsafe{ ::core::mem::transmute(::core::raw::Slice::<ModuleInfo> {
		data: baseptr,
		len: count,
		})};
	log_debug!("s_modules={},{:#x}", mods.as_ptr(), mods.len());
	for (i,module) in mods.iter().enumerate()
	{
		log_debug!("#{}: {}", i, module.name);
		(module.init)();
	}
}

// vim: ft=rust

