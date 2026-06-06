//! Shared font-library state.
//!
//! Upstream maps CoreText-family backends to `NoopLibrary`: CoreText does not
//! require process-wide FreeType-style library state. Roastty currently targets
//! that CoreText path, so its library boundary is intentionally zero-sized.

/// Process-wide font library state for the active backend.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Library;

impl Library {
    /// Create the active backend's library state.
    pub(crate) const fn new() -> Library {
        Library
    }

    /// Upstream-compatible constructor name.
    pub(crate) const fn init() -> Library {
        Library::new()
    }

    /// Deinitialize the library state.
    pub(crate) fn deinit(self) {
        let _ = self;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_new_and_init_are_infallible_noops() {
        assert_eq!(Library::new(), Library::init());
        Library::new().deinit();
        Library::init().deinit();
    }

    #[test]
    fn library_is_zero_sized_and_copyable() {
        assert_eq!(std::mem::size_of::<Library>(), 0);
        let a = Library::new();
        let b = a;
        let c = a;
        assert_eq!(a, b);
        assert_eq!(b, c);
    }
}
