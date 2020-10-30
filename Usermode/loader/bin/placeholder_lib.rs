#[no_mangle] pub extern "C" fn new_process() { loop{} }
#[no_mangle] pub extern "C" fn start_process() { loop{} }

#[cfg(arch="native")]
pub mod _foo {
    #[no_mangle] pub extern "C" fn rustos_native_init() { loop {} }
    #[no_mangle] pub extern "C" fn rustos_native_syscall() { loop {} }
}