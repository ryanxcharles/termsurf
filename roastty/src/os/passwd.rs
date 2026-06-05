//! The passwd database entry for the current user (port of upstream `os/passwd`).

use std::ffi::{CStr, OsString};
use std::os::unix::ffi::OsStringExt;

/// The passwd fields we care about for the current user (upstream `os.passwd.Entry`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) shell: Option<OsString>,
    pub(crate) home: Option<OsString>,
    pub(crate) name: Option<OsString>,
}

/// Get the passwd entry for the currently executing user (upstream `os.passwd.get`). On any
/// lookup failure (non-zero `getpwuid_r` or a null result) an empty `Entry` is returned.
pub(crate) fn get() -> Entry {
    let mut buf = [0 as libc::c_char; 1024];
    let mut pw: libc::passwd = unsafe { std::mem::zeroed() };
    let mut pw_ptr: *mut libc::passwd = std::ptr::null_mut();

    let res = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut pw,
            buf.as_mut_ptr(),
            buf.len(),
            &mut pw_ptr,
        )
    };
    // A non-zero return or a null entry means "no entry"; upstream logs and returns empty.
    if res != 0 || pw_ptr.is_null() {
        return Entry::default();
    }

    Entry {
        shell: cstr_to_os(pw.pw_shell),
        home: cstr_to_os(pw.pw_dir),
        name: cstr_to_os(pw.pw_name),
    }
}

/// Copy a (possibly null) NUL-terminated C string field into an owned `OsString`.
fn cstr_to_os(ptr: *const libc::c_char) -> Option<OsString> {
    if ptr.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes().to_vec();
    Some(OsString::from_vec(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_user_has_shell_home_and_name() {
        let entry = get();

        let shell = entry.shell.expect("shell present");
        assert!(!shell.is_empty());

        let home = entry.home.expect("home present");
        assert!(!home.is_empty());

        let name = entry.name.expect("name present");
        assert!(!name.is_empty());
    }
}
