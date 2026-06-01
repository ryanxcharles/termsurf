//! Terminal device status requests.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Request {
    OperatingStatus,
    CursorPosition,
    ColorScheme,
}

pub(super) fn request_from_int(value: u16, question: bool) -> Option<Request> {
    match (value, question) {
        (5, false) => Some(Request::OperatingStatus),
        (6, false) => Some(Request::CursorPosition),
        (996, true) => Some(Request::ColorScheme),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_status_request_from_int_matches_supported_queries() {
        assert_eq!(request_from_int(5, false), Some(Request::OperatingStatus));
        assert_eq!(request_from_int(6, false), Some(Request::CursorPosition));
        assert_eq!(request_from_int(996, true), Some(Request::ColorScheme));

        assert_eq!(request_from_int(5, true), None);
        assert_eq!(request_from_int(6, true), None);
        assert_eq!(request_from_int(996, false), None);
        assert_eq!(request_from_int(999, false), None);
    }
}
