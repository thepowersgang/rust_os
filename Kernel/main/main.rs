// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// main/main.rs
// - Kernel main and initialisation
#![crate_name="main"]
#![crate_type="rlib"]
#![no_std]
#![feature(negative_impls)]	// Used for !Send on LoaderHeader (for pedantic safety)
#![feature(asm)]

#[macro_use]
extern crate kernel;
extern crate syscalls;

#[cfg(not(target))]
pub mod modules {
	fn use_mod(m: &::kernel::modules::ModuleInfo) {
		unsafe { asm!("" : : "r" (m)) }
	}
	pub fn use_mods() -> usize {
		let mut rv = 0;
		include!{ concat!( env!("OUT_DIR"), "/modules.rs" ) }
		rv
	}
}
#[cfg(target)]
pub mod modules {
	pub fn use_mods() -> usize {
		0
	}
}

/// Kernel entrypoint
#[no_mangle]
pub extern "C" fn kmain()
{
	log_notice!("{} starting", ::kernel::VERSION_STRING);
	log_notice!("> {}", ::kernel::BUILD_STRING);
	log_notice!("{} compiled-in modules", modules::use_mods());
	
	// Initialise core services before attempting modules
	::kernel::memory::phys::init();
	::kernel::memory::virt::init();
	::kernel::memory::heap::init();
	::kernel::memory::page_cache::init();
	::kernel::threads::init();
	
	log_log!("Command line = '{}'", ::kernel::arch::boot::get_boot_string());
	::kernel::config::init( ::kernel::arch::boot::get_boot_string() );
	
	// Dump active video mode
	let vidmode = ::kernel::arch::boot::get_video_mode();
	match vidmode {
	Some(m) => {
		log_debug!("Video mode : {}x{} @ {:#x}", m.width, m.height, m.base);
		::kernel::metadevs::video::set_boot_mode(m);
		},
	None => log_debug!("No video mode present")
	}
	
	// Intialise the IRQ worker
	::kernel::irqs::init();
	
	// Modules (dependency tree included)
	// - Requests that the GUI be started as soon as possible
	::kernel::modules::init(&["GUI"]);
	
	// Yield to allow init threads to run
	//::kernel::threads::yield_time();
	
	// Run system init
	sysinit();
}

// Initialise the system once drivers are up
fn sysinit() -> !
{
	use kernel::metadevs::storage::VolumeHandle;
	use kernel::vfs::{mount,handle};
	use kernel::vfs::Path;
	use kernel::config::{get_string, Value};

	let test_flags = get_string(Value::TestFlags);
	if test_flags.split(',').any(|v| v == "noinit")
	{
		log_error!("Stopping at sysinit");
		::kernel::threads::SleepObject::with_new("noinit", |so| so.wait());
	}
	
	// 1. Mount /system to the specified volume
	let sysdisk = ::kernel::config::get_string(::kernel::config::Value::SysDisk);
	match VolumeHandle::open_named(sysdisk)
	{
	Err(e) => {
		panic!("Unable to open /system volume {}: {}", sysdisk, e);
		},
	Ok(vh) => match mount::mount("/system".as_ref(), vh, "", &[])
		{
		Ok(_) => {},
		Err(e) => {
			panic!("Unable to mount /system from {}: {:?}", sysdisk, e);
			},
		},
	}
	
	// 2. Symbolic link /sysroot to the specified folder
	let sysroot = ::kernel::config::get_string(::kernel::config::Value::SysRoot);
	log_debug!("sysroot = \"{}\"", sysroot);
	handle::Dir::open(Path::new("/")).unwrap()
		.symlink("sysroot", Path::new(&sysroot[..])).unwrap();
	
	vfs_test();
	
	// 3. Start 'init' (root process) using the userland loader
	let loader = ::kernel::config::get_string(::kernel::config::Value::Loader);
	let init = ::kernel::config::get_string(::kernel::config::Value::Init);
	match spawn_init(loader, init)
	{
	Ok(_) => unreachable!(),
	Err(e) => panic!("Failed to start init: {}", e),
	}
}

//#[cfg(DISABLED)]
fn vfs_test()
{
	use kernel::vfs::handle;
	use kernel::vfs::Path;
	
	fn ls(p: &Path) {
		// - Iterate root dir
		log_log!("ls({:?})", p);
		match handle::Dir::open(p)
		{
		Err(e) => log_warning!("'{:?}' cannot be opened: {:?}", p, e),
		Ok(h) =>
			for name in h.iter() {
				log_log!("{:?}", name);
			},
		}
	}

	// *. Testing: open a file known to exist on the testing disk	
	if false
	{
		match handle::File::open( Path::new("/system/1.TXT"), handle::FileOpenMode::SharedRO )
		{
		Err(e) => log_warning!("VFS test file can't be opened: {:?}", e),
		Ok(h) => {
			log_debug!("VFS open test = {:?}", h);
			let mut buf = [0; 16];
			let sz = h.read(0, &mut buf).unwrap();
			log_debug!("- Contents: {:?}", ::kernel::lib::RawString(&buf[..sz]));
			},
		}
		
		ls(Path::new("/"));
		ls(Path::new("/system"));
	}
	
	// *. TEST Automount
	// - Probably shouldn't be included in the final version, but works for testing filesystem and storage drivers
	automount();

	ls(Path::new("/mount/ahci?-0p0"));
}
fn automount()
{
	use kernel::metadevs::storage::VolumeHandle;
	use kernel::vfs::{Path,mount,handle};

	let mountdir = handle::Dir::open( Path::new("/") ).and_then(|h| h.mkdir("mount")).unwrap();
	for (_,v) in ::kernel::metadevs::storage::enum_lvs()
	{
		let vh = match VolumeHandle::open_named(&v)
			{
			Err(e) => {
				log_log!("Unable to open '{}': {}", v, e);
				continue;
				},
			Ok(v) => v,
			};
		mountdir.mkdir(&v).unwrap();
		let mountpt = format!("/mount/{}",v);
		match mount::mount( mountpt.as_ref(), vh, "", &[] )
		{
		Ok(_) => log_log!("Auto-mounted to {}", mountpt),
		Err(e) => log_notice!("Unable to automount '{}': {:?}", v, e),
		}
	}
}

fn spawn_init(loader_path: &str, init_cmdline: &str) -> Result<::kernel::Void, &'static str>
{
	use kernel::vfs::handle;
	use kernel::vfs::Path;
	
	
	log_log!("Loading userland '{}' args '{}'", loader_path, init_cmdline);
	
	// - 1. Memory-map the loader binary to a per-architecture location
	//  > E.g. for x86 it'd be 0xBFFF0000 - Limiting it to 64KiB
	//  > For amd64: 1<<48-64KB
	//  > PANIC if the binary (or its memory size) is too large
	let loader = match handle::File::open(Path::new(loader_path), handle::FileOpenMode::Execute)
		{
		Ok(v) => v,
		Err(e) => {
			log_error!("Unable to open initial userland loader '{}': {:?}", loader_path, e);
			return Err("No such file");
			},
		};
	// TODO: Split init_cmdline on space to allow argument passing
	let init = match handle::File::open(Path::new(init_cmdline), handle::FileOpenMode::Execute)
		{
		Ok(v) => v,
		Err(e) => {
			log_error!("Unable to open userland init '{}': {:?}", init_cmdline, e);
			return Err("No such file");
			},
		};
	
	// - Load the loader into memory
	let (header_ptr, memsize) = load_loader(&loader)?;

	// - Populate argument region and return size written
	// SAFE: Addresses are checked by load_loader
	let argslen = unsafe {
		let init_path_ofs = header_ptr.init_path_ofs as usize;
		let init_path_len = header_ptr.init_path_len as usize;
		assert!(init_path_ofs <= memsize);
		assert!(init_path_ofs + init_path_len <= memsize);
		let cmdline_buf_base = LOAD_BASE + init_path_ofs;
		let cmdline_buf = ::core::slice::from_raw_parts_mut(cmdline_buf_base as *mut u8, init_path_len);
		cmdline_buf[..init_cmdline.len()].clone_from_slice( init_cmdline.as_bytes() );
		init_cmdline.len()
		};
	
	// - 6. Enter userland
	if ! ( LOAD_BASE <= header_ptr.entrypoint && header_ptr.entrypoint < LOAD_MAX ) {
		log_error!("Userland entrypoint out of range: {:#x}", header_ptr.entrypoint);
		return Err("Loader invalid");
	}
	
	::syscalls::init(loader, init);
	
	log_notice!("Entering userland at {:#x} '{}' '{}'", header_ptr.entrypoint, loader_path, init_cmdline);
	// SAFE: This pointer is as validated as it can be...
	unsafe {
		::kernel::arch::drop_to_user(header_ptr.entrypoint, 0, argslen);
	}
}

fn load_loader(loader: &::kernel::vfs::handle::File) -> Result<(&'static LoaderHeader, usize), &'static str>
{
	use core::mem::forget;
	use kernel::vfs::handle;
	use kernel::PAGE_SIZE;

	let ondisk_size = loader.size();
	let mh_firstpage = {
		if ondisk_size > MAX_SIZE as u64 {
			log_error!("Loader is too large to fit in reserved region ({}, max {})",
				ondisk_size, MAX_SIZE);
			return Err("Loader too large");
		}
		loader.memory_map(LOAD_BASE,  0, PAGE_SIZE,  handle::MemoryMapMode::Execute).expect("Loader first page")
		};
	// - 2. Parse the header
	// SAFE: LoaderHeader is POD, and pointer is valid (not Sync, so passing to another thread/process is invalid)
	let header_ptr = unsafe { &*(LOAD_BASE as *const LoaderHeader) };
	if header_ptr.magic != MAGIC || header_ptr.info != INFO {
		log_error!("Loader header is invalid: magic {:#x} != {:#x} or info {:#x} != {:#x}",
			header_ptr.magic, MAGIC, header_ptr.info, INFO);
		return Err("Loader invalid");
	}
	// - 3. Map the remainder of the image into memory (with correct permissions)
	let codesize = header_ptr.codesize as usize;
	let memsize = header_ptr.memsize as usize;
	let datasize = ondisk_size as usize - codesize;
	let bss_size = memsize - ondisk_size as usize;
	log_debug!("Executable size: {}, rw data size: {}", codesize, datasize);
	assert!(codesize % PAGE_SIZE == 0, "Loader code doesn't end on a page boundary - {:#x}", codesize);
	assert!(ondisk_size as usize % PAGE_SIZE == 0, "Loader file size is not aligned to a page - {:#x}", ondisk_size);
	assert!(datasize % PAGE_SIZE == 0, "Loader is not an integeral number of pages long - datasize={:#x}", datasize);
	let mh_code = loader.memory_map(LOAD_BASE + PAGE_SIZE, PAGE_SIZE as u64, codesize - PAGE_SIZE,  handle::MemoryMapMode::Execute).expect("Loader code");
	let mh_data = loader.memory_map(LOAD_BASE + codesize, codesize as u64, datasize,  handle::MemoryMapMode::COW).expect("Loader data");
	
	// - 4. Allocate the loaders's BSS
	let pages = (bss_size + PAGE_SIZE-1) / PAGE_SIZE;
	let bss_start = (LOAD_BASE + ondisk_size as usize) as *mut ();
	let ah_bss = ::kernel::memory::virt::allocate_user(bss_start, pages);
	
	// - 5. Write loader arguments
	//   > Target buffer should be outside of the code region, and within the reserved region
	if header_ptr.init_path_ofs as usize > codesize && (header_ptr.init_path_ofs as usize + header_ptr.init_path_len as usize) <= memsize {
		// Init commandline is within a valid region
		// TODO: Should this function return a slice instead of letting the caller do the casts?
	}
	else {
		log_error!("Userland init string location out of range: {:#x}+{} not in {:#x}--{:#x}", header_ptr.init_path_ofs, header_ptr.init_path_len, codesize, memsize);
		return Err("Loader invalid");
	}

	// > Forget about all maps and allocations
	forget(mh_firstpage);
	forget(mh_code);
	forget(mh_data);
	forget(ah_bss);


	Ok( (header_ptr, memsize) )
}

#[repr(C)]
struct LoaderHeader
{
	magic: u32,
	info: u32,
	codesize: u32,
	memsize: u32,
	init_path_ofs: u32,
	init_path_len: u32,
	entrypoint: usize,
}
impl !Sync for LoaderHeader { }
#[allow(dead_code)]
#[repr(u8)]
enum ArchValues {
	X86 = 1,
	AMD64 = 2,
	ARMv7 = 3,
	ARMv8 = 4,
}
#[cfg(any(arch="amd64",target_arch="x86_64"))]	const ARCH: ArchValues = ArchValues::AMD64;
#[cfg(any(arch="amd64",target_arch="x86_64"))]	const LOAD_MAX: usize = 1 << 47;
#[cfg(arch="armv7")]	const ARCH: ArchValues = ArchValues::ARMv7;
#[cfg(arch="armv7")]	const LOAD_MAX: usize = (1 << 31) - (4 << 20);	// Leave 4MB for the kernel to control within the user table
#[cfg(arch="armv8")]	const ARCH: ArchValues = ArchValues::ARMv8;
#[cfg(arch="armv8")]	const LOAD_MAX: usize = (1 << 48) - (64 << 30);	// Leave 64GB for the kernel to control within the user table
#[cfg(target_pointer_width="64")]	const USIZE_BYTES: u32 = 8;
#[cfg(target_pointer_width="32")]	const USIZE_BYTES: u32 = 4;
const MAGIC: u32 = 0x71FF1013;
const INFO: u32 = (5*4 + USIZE_BYTES) | ((ARCH as u8 as u32) << 8);

const MAX_SIZE: usize = 4*64*1024;	// 128KB
const LOAD_BASE: usize = LOAD_MAX - MAX_SIZE;
