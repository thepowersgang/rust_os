// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/config.rs
//! Boot-time configuration management
// NOTE: See the bottom of the file for the runtime configuration options
#![allow(static_mut_refs)]

/// Initialise the kerel config from a command-line string.
/// 
/// UNSAFE: Needs to be called in a single-threaded context, as it mutates a `static mut``
pub unsafe fn init(cmdline: &'static str)
{
	S_CONFIG.init(cmdline);
}

/// Get a configuration parameter from the kernel config
pub fn get_string(val: Value) -> &'static str
{
	// SAFE: No mutation should happen when get_string is being called
	unsafe {
		S_CONFIG.get_str(val)
	}
}


macro_rules! def_config_set {
	(
		$enum_name:ident in $struct_name:ident : {
			$(
			$(#[$at:meta])*
			$name:ident @ $sname:pat = $default:expr,
			)*
		}
	) => {
		#[allow(non_snake_case)]
		struct $struct_name {
			$($name: Option<&'static str>),*
		}
		pub enum $enum_name {
			$( $(#[$at])* $name, )*
		}
		impl Config
		{
			const fn new() -> Config {
				Config { $($name: None),* }
			}
			
			fn init(&mut self, cmdline: &'static str)
			{
				for ent in cmdline.split(' ')
				{
					let mut it = ent.splitn(2, '=');
					let tag = it.next().unwrap();
					let value = it.next();
					match tag
					{
					$(
					$sname => match value
						{
						Some(v) => self.$name = Some(v),
						None => log_warning!("{} requires a value", tag),
						},
					)*
					v @ _ => log_warning!("Unknown option '{}", v),
					}
				}
			}

			fn get_str(&self, val: Value) -> &'static str
			{
				match val
				{
				$(
				Value::$name => self.$name.unwrap_or($default),
				)*
				}
			}
		}
	};
}

def_config_set! {
	Value in Config: {
		/// VFS - Volume to mount as the 'system' disk
		SysDisk @ "SYSDISK" = "ATA0p0",
//		/// VFS - Path relative to the root of SysDisk where Tifflin was installed
		SysRoot @ "SYSROOT" = "/system/Tifflin",
//		/// Startup - Loader executable
		Loader @ "LOADER" = "/sysroot/bin/loader",
//		/// Startup - Init executable (first userland process)
		Init @ "INIT" = "/sysroot/bin/init",
		TestFlags @ "TEST" = "",
	}
}

static mut S_CONFIG: Config = Config::new();

