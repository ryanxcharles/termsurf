//! A small-buffer-optimization message payload (port of upstream `datastruct/message_data`).

/// A message payload held inline (`Small`), borrowed (`Stable`), or heap-allocated (`Alloc`)
/// (upstream `datastruct.MessageData`). `Small` avoids allocation when the payload fits.
pub(crate) enum MessageData<'a, T: Copy + Default, const SMALL: usize> {
    /// The payload copied into a fixed inline array; only `data[..len]` is meaningful.
    Small { data: [T; SMALL], len: usize },
    /// A borrowed "stable" slice passed through directly (e.g. `const` data).
    Stable(&'a [T]),
    /// An owned, heap-allocated payload (freed on drop).
    Alloc(Vec<T>),
}

impl<'a, T: Copy + Default, const SMALL: usize> MessageData<'a, T, SMALL> {
    /// Build a message from `data`, fitting it inline when it fits, else allocating (upstream
    /// `init`). Never produces `Stable`.
    pub(crate) fn init(data: &[T]) -> MessageData<'static, T, SMALL> {
        if data.len() <= SMALL {
            let mut buf = [T::default(); SMALL];
            buf[..data.len()].copy_from_slice(data);
            MessageData::Small {
                data: buf,
                len: data.len(),
            }
        } else {
            MessageData::Alloc(data.to_vec())
        }
    }

    /// Wrap a borrowed "stable" slice (the `Stable` variant; `init` never produces this).
    pub(crate) fn stable(data: &'a [T]) -> Self {
        MessageData::Stable(data)
    }

    /// A read-only view of the payload (upstream `slice`).
    pub(crate) fn slice(&self) -> &[T] {
        match self {
            MessageData::Small { data, len } => &data[..*len],
            MessageData::Stable(s) => s,
            MessageData::Alloc(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_small_fits_inline() {
        let data = MessageData::<u8, 10>::init(b"hello!");
        assert!(matches!(data, MessageData::Small { .. }));
        assert_eq!(data.slice(), b"hello!");
    }

    #[test]
    fn init_large_allocates() {
        let input = b"hello! ".repeat(100); // 700 bytes
        let data = MessageData::<u8, 10>::init(&input);
        assert!(matches!(data, MessageData::Alloc(_)));
        assert_eq!(data.slice(), input.as_slice());
    }

    #[test]
    fn init_at_capacity_fits_inline() {
        let input = vec![b'X'; 500];
        let data = MessageData::<u8, 500>::init(&input);
        assert!(matches!(data, MessageData::Small { .. }));
        assert_eq!(data.slice(), input.as_slice());
    }

    #[test]
    fn stable_wraps_borrowed_slice() {
        let data: MessageData<u8, 10> = MessageData::stable(b"const");
        assert!(matches!(data, MessageData::Stable(_)));
        assert_eq!(data.slice(), b"const");
    }
}
