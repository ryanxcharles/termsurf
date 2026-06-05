//! macOS-specific helpers (port of upstream `os/macos`).

use std::ffi::CStr;

/// Set the name of the **running** thread (upstream `os.macos.pthread_setname_np`). On macOS
/// `pthread_setname_np` names the calling thread; the name is limited to `MAXTHREADNAMESIZE`
/// (64 bytes including the NUL), and an over-long name fails with `ENAMETOOLONG`.
pub(crate) fn set_thread_name(name: &CStr) -> std::io::Result<()> {
    // Returns 0 on success, -1 with `errno` set on failure (runtime-verified on this macOS
    // SDK: a 100-byte name yields rc = -1, errno = ENAMETOOLONG). Unlike
    // `pthread_set_qos_class_self_np`, this is the `-1`/`errno` convention, so read `errno`.
    let rc = unsafe { libc::pthread_setname_np(name.as_ptr()) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// The macOS thread quality-of-service levels (upstream `os.macos.QosClass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub(crate) enum QosClass {
    UserInteractive = 0x21,
    UserInitiated = 0x19,
    Default = 0x15,
    Utility = 0x11,
    Background = 0x09,
    Unspecified = 0x00,
}

impl QosClass {
    fn to_libc(self) -> libc::qos_class_t {
        match self {
            QosClass::UserInteractive => libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE,
            QosClass::UserInitiated => libc::qos_class_t::QOS_CLASS_USER_INITIATED,
            QosClass::Default => libc::qos_class_t::QOS_CLASS_DEFAULT,
            QosClass::Utility => libc::qos_class_t::QOS_CLASS_UTILITY,
            QosClass::Background => libc::qos_class_t::QOS_CLASS_BACKGROUND,
            QosClass::Unspecified => libc::qos_class_t::QOS_CLASS_UNSPECIFIED,
        }
    }
}

/// An error setting the thread QoS class (upstream `os.macos.SetQosClassError`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SetQosClassError {
    /// The thread can't have its QoS class changed (usually because a different pthread API
    /// made it an invalid target).
    ThreadIncompatible,
}

/// Set the QoS class of the running thread (upstream `os.macos.setQosClass`).
pub(crate) fn set_qos_class(class: QosClass) -> Result<(), SetQosClassError> {
    let rc = unsafe { libc::pthread_set_qos_class_self_np(class.to_libc(), 0) };
    map_qos_result(rc)
}

/// Map a `pthread_set_qos_class_self_np` return code to a result. The function returns **zero
/// on success, otherwise an errno value directly** (per the Apple `<pthread/qos.h>` docs) —
/// it does *not* use the `-1`/`errno` convention, so the code is matched directly.
fn map_qos_result(rc: libc::c_int) -> Result<(), SetQosClassError> {
    match rc {
        0 => Ok(()),
        // EPERM is the only known error per the man page.
        libc::EPERM => Err(SetQosClassError::ThreadIncompatible),
        _ => panic!("unexpected pthread_set_qos_class_self_np error"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qos_class_discriminants_match_upstream() {
        for (class, value) in [
            (QosClass::UserInteractive, 0x21u32),
            (QosClass::UserInitiated, 0x19),
            (QosClass::Default, 0x15),
            (QosClass::Utility, 0x11),
            (QosClass::Background, 0x09),
            (QosClass::Unspecified, 0x00),
        ] {
            assert_eq!(class as u32, value);
            assert_eq!(class.to_libc() as u32, value);
        }
    }

    #[test]
    fn map_qos_result_maps_codes() {
        assert_eq!(map_qos_result(0), Ok(()));
        assert_eq!(
            map_qos_result(libc::EPERM),
            Err(SetQosClassError::ThreadIncompatible),
        );
    }

    #[test]
    #[should_panic(expected = "unexpected pthread_set_qos_class_self_np error")]
    fn map_qos_result_panics_on_unexpected_errno() {
        let _ = map_qos_result(libc::EINVAL);
    }

    #[test]
    fn set_qos_class_succeeds_on_normal_thread() {
        // A cargo test thread is a normal pthread, so setting its QoS is benign and succeeds.
        assert_eq!(set_qos_class(QosClass::Default), Ok(()));
    }

    fn current_thread_name() -> std::ffi::CString {
        let mut buf = [0 as libc::c_char; 64];
        let rc =
            unsafe { libc::pthread_getname_np(libc::pthread_self(), buf.as_mut_ptr(), buf.len()) };
        assert_eq!(rc, 0, "pthread_getname_np failed");
        unsafe { CStr::from_ptr(buf.as_ptr()) }.to_owned()
    }

    #[test]
    fn set_thread_name_round_trips() {
        let name = c"roastty-552";
        set_thread_name(name).expect("set thread name");
        assert_eq!(current_thread_name().as_c_str(), name);
    }

    #[test]
    fn set_thread_name_too_long_is_enametoolong() {
        // Name the thread first so we have a known value to confirm it is left unchanged.
        let original = c"roastty-552-orig";
        set_thread_name(original).expect("set thread name");

        let long = std::ffi::CString::new("a".repeat(100)).unwrap();
        let err = set_thread_name(&long).expect_err("over-long name should fail");
        assert_eq!(err.raw_os_error(), Some(libc::ENAMETOOLONG));

        // A failed set leaves the thread name unchanged.
        assert_eq!(current_thread_name().as_c_str(), original);
    }
}
