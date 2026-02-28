# Issue 676: URL Normalization

Automatically prepend `https://` when the user types a bare domain like
`google.com` instead of `https://google.com`.

## Background

Currently the TUI passes URLs verbatim to Chromium. If the user types
`web google.com` or enters `google.com` in the URL bar, Chromium receives
`google.com` as-is, which may fail or trigger a search instead of navigation.

Users expect browser-like behavior: type `google.com`, get `https://google.com`.

## Experiment 1: Prepend https:// for bare domains

### Hypothesis

A simple `normalize_url` function in the TUI that detects bare domains (contains
a dot, no scheme) and prepends `https://` will give browser-like URL entry.

### Changes

#### 1. main.rs — add `normalize_url` function

```rust
fn normalize_url(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.contains("://") {
        return trimmed.to_string();
    }
    if trimmed.contains('.') {
        return format!("https://{trimmed}");
    }
    trimmed.to_string()
}
```

The heuristic:

- Already has a scheme (`://`) — pass through (`https://google.com`,
  `file:///tmp/foo.html`)
- Contains a dot — treat as domain, prepend `https://` (`google.com` →
  `https://google.com`)
- No dot, no scheme — pass through as-is (could be a search query in the future)

#### 2. main.rs — apply at both URL entry points

**CLI resolution** (after the `url` match):

```rust
let mut url = normalize_url(&url);
```

**Editor submit** (Edit mode, Enter key):

```rust
url = normalize_url(&new_url);
```

### Test

1. `cd tui && cargo build` — compiles without errors
2. `web google.com` → navigates to `https://google.com`
3. `web https://google.com` → still works (scheme preserved)
4. `web http://localhost:3000` → still works (http preserved)
5. `web file:///tmp/test.html` → still works (file scheme preserved)
6. Type `github.com` in URL bar, press Enter → `https://github.com`
7. `web` (no args) → homepage unchanged (already has scheme)
