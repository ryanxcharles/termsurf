mod ffi;

use std::ffi::c_void;
use std::ptr;

unsafe extern "C" fn on_initialized(_user_data: *mut c_void) {
    eprintln!("[Roamium] Chromium initialized — creating browser context");
    let ctx = unsafe { ffi::ts_create_browser_context(ptr::null()) };
    eprintln!("[Roamium] Browser context: {:?}", ctx);
    eprintln!("[Roamium] Smoke test passed — shutting down");
    unsafe { ffi::ts_quit() };
}

fn main() {
    let args: Vec<std::ffi::CString> = std::env::args()
        .map(|a| std::ffi::CString::new(a).unwrap())
        .collect();
    let argv: Vec<*const i8> = args.iter().map(|a| a.as_ptr()).collect();

    unsafe {
        ffi::ts_set_on_initialized(Some(on_initialized), ptr::null_mut());
    }

    eprintln!("[Roamium] Entering ts_content_main");
    let ret = unsafe { ffi::ts_content_main(argv.len() as i32, argv.as_ptr()) };
    std::process::exit(ret);
}
