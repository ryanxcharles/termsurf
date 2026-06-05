//! Command-line argument iterator.
//!
//! The macOS product path mirrors upstream `os.args.iterator`: read process
//! arguments from `NSProcessInfo.arguments` so app launches that do not have a
//! reliable libc `argc`/`argv` still see their startup arguments.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Args {
    values: Vec<String>,
    index: usize,
}

impl Args {
    pub(crate) fn new(values: Vec<String>) -> Self {
        Self { values, index: 0 }
    }

    pub(crate) fn from_process_info() -> Self {
        Self::new(process_info_arguments())
    }

    pub(crate) fn next(&mut self) -> Option<&str> {
        let value = self.values.get(self.index)?;
        self.index += 1;
        Some(value.as_str())
    }

    pub(crate) fn skip(&mut self) -> bool {
        if self.index == self.values.len() {
            return false;
        }

        self.index += 1;
        true
    }

    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }

    pub(crate) fn remaining(&self) -> usize {
        self.values.len().saturating_sub(self.index)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

pub(crate) fn iterator() -> Args {
    Args::from_process_info()
}

#[cfg(target_os = "macos")]
fn process_info_arguments() -> Vec<String> {
    use objc2::rc::autoreleasepool;
    use objc2_foundation::NSProcessInfo;

    let info = NSProcessInfo::processInfo();
    let args = info.arguments();

    autoreleasepool(|pool| {
        (0..args.len())
            .map(|index| {
                let value = args.objectAtIndex(index);
                unsafe { value.to_str(pool) }.to_owned()
            })
            .collect()
    })
}

#[cfg(not(target_os = "macos"))]
fn process_info_arguments() -> Vec<String> {
    std::env::args().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn next_yields_snapshot_values_in_order() {
        let mut args = Args::new(strings(&["app", "--flag", "value"]));

        assert_eq!(args.len(), 3);
        assert!(!args.is_empty());
        assert_eq!(args.remaining(), 3);
        assert_eq!(args.next(), Some("app"));
        assert_eq!(args.remaining(), 2);
        assert_eq!(args.next(), Some("--flag"));
        assert_eq!(args.next(), Some("value"));
        assert_eq!(args.next(), None);
        assert_eq!(args.remaining(), 0);
    }

    #[test]
    fn skip_advances_one_argument() {
        let mut args = Args::new(strings(&["app", "--skip", "kept"]));

        assert!(args.skip());
        assert_eq!(args.remaining(), 2);
        assert_eq!(args.next(), Some("--skip"));
        assert!(args.skip());
        assert!(!args.skip());
        assert_eq!(args.next(), None);
    }

    #[test]
    fn empty_snapshot_has_no_values_to_yield_or_skip() {
        let mut args = Args::new(Vec::new());

        assert_eq!(args.len(), 0);
        assert!(args.is_empty());
        assert_eq!(args.remaining(), 0);
        assert_eq!(args.next(), None);
        assert!(!args.skip());
    }

    #[test]
    fn process_iterator_smoke_test_has_program_argument() {
        let mut args = iterator();

        assert!(!args.is_empty());
        assert!(args.next().is_some());
    }
}
