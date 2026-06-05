//! macOS-specific helpers (port of upstream `os/macos`).

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
}
