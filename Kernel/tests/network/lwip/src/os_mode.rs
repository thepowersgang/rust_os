//! Hosted

/// HELPER: Create a callback function that calls the provided function
/// 
/// Stores the data inline in the pointer if possible, otherwise boxes
/// 
/// NOTE: The returned function must only ever be called once.
pub fn create_callback<F>(callback: F) -> (extern "C" fn(*mut ::std::ffi::c_void), *mut ::std::ffi::c_void)
where
    F: FnOnce() + 'static
{
    if ::core::mem::size_of_val(&callback) < ::std::mem::size_of::<usize>() {
        let d: *mut ::std::os::raw::c_void = unsafe { ::std::mem::transmute_copy(&callback) };
        extern "C" fn raw_cb<F: FnOnce()>(data: *mut ::std::os::raw::c_void) {
            let icb: F = unsafe { ::std::mem::transmute_copy(&data) };
            (icb)();
        }
        (raw_cb::<F> as extern "C" fn(_), d)
    }
    else {
        let d: *mut ::std::os::raw::c_void = Box::into_raw(Box::new(callback)) as *mut _;
        extern "C" fn raw_cb<F: FnOnce()>(data: *mut ::std::os::raw::c_void) {
            // SAFE: This should only ever be called once
            let icb: Box<F> = unsafe { Box::from_raw(data as *mut _) };
            (icb)();
        }
        (raw_cb::<F> as extern "C" fn(_), d)
    }
}

/// `tcpip_init`
pub fn init<F: FnMut() + 'static>(callback: F) {
    let (raw_cb, data) = create_callback(callback);
    unsafe {
        ::lwip_sys::tcpip_init(Some(raw_cb), data)
    }
}

/// `tcpip_callback` - Call a method on the tcpip thread
pub fn callback<F: FnOnce() + 'static>(callback: F) {
    let (raw_cb, data) = create_callback(callback);
    unsafe {
        ::lwip_sys::tcpip_callback(Some(raw_cb), data);
    }
}

