extension TermSurf {
    /// Possible errors from internal TermSurf calls.
    enum Error: Swift.Error, CustomLocalizedStringResourceConvertible {
        case apiFailed

        var localizedStringResource: LocalizedStringResource {
            switch self {
            case .apiFailed: return "libtermsurf API call failed"
            }
        }
    }
}
