use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

pub type TsBrowserContext = *mut c_void;
pub type TsWebContents = *mut c_void;

extern "C" {
    pub fn ts_content_main(argc: c_int, argv: *const *const c_char) -> c_int;
    pub fn ts_set_on_initialized(
        callback: Option<unsafe extern "C" fn(*mut c_void)>,
        user_data: *mut c_void,
    );
    pub fn ts_post_task(
        task: Option<unsafe extern "C" fn(*mut c_void)>,
        user_data: *mut c_void,
    );
    pub fn ts_quit();
    pub fn ts_create_browser_context(path: *const c_char) -> TsBrowserContext;
}
