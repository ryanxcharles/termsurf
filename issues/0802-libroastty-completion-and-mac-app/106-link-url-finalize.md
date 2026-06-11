# Experiment 106: Phase F — link-url finalize

## Description

Port the next upstream config-default/finalize behavior after Experiment 105:
the default URL/path matcher and `link-url = false` removal of that matcher.

Upstream `Config.default()` appends one default link before any user config is
loaded:

```zig
try result.link.links.append(alloc, .{
    .regex = url.regex,
    .action = .{ .open = {} },
    .highlight = .{ .hover_mods = inputpkg.ctrlOrSuper(.{}) },
});
```

Later, `Config.finalize()` removes that first matcher when `link-url` is false:

```zig
if (!self.@"link-url") self.link.links.items = self.link.links.items[1..];
```

Roastty already has the `link-url` boolean and an `input::link::Link` type, but
`Config` does not yet own upstream's repeatable link list. Upstream's
`RepeatableLink.parseCLI` is still `NotImplemented` at the pinned Ghostty
commit, so this experiment should add only the default link list and finalize
mutation needed for upstream parity at this stage.

This is a config-internal slice. It must not implement user `link = ...`
parsing, regex compilation/matching, renderer `link_ranges`, link preview UI,
open-url dispatch, app C ABI exposure, key-remap finalization, or a broader
runtime link-highlighting system.

## Changes

- `roastty/src/config/mod.rs`
  - Add config-owned storage for the upstream repeatable link list, using the
    existing `crate::input::link::Link` type.
  - Add a local port of the pinned upstream default URL/path regex bytes from
    `vendor/ghostty/src/config/url.zig`, with comments tying it to the pinned
    source and making clear it is data for the default config matcher, not a
    complete link-highlighting implementation.
  - Initialize `Config::default()` with one default link:
    - `regex` equal to the pinned upstream URL/path regex;
    - `action = Open`;
    - `highlight = HoverMods(ctrl_or_super(Mods::new()))`.
  - During scalar finalization, after window size clamping and before the
    quit-delay warning / auto-update tail, remove the first link when
    `link_url == false`.
  - Preserve the default link when `link_url == true`.
  - Do not parse, format, or expose user-configured `link` entries in this
    experiment. The formatter should continue to omit `link = ...`, matching
    upstream's currently unimplemented `RepeatableLink.formatEntry`.
  - Add focused tests proving:
    - `Config::default()` contains exactly the default URL matcher with the
      expected action and highlight;
    - the default link regex is byte-for-byte equal to the complete local pinned
      upstream URL/path regex constant, not only representative fragments;
    - `link-url = true` preserves that default matcher through finalization;
    - `link-url = false` removes the default matcher through finalization;
    - removing the matcher does not skip later scalar finalization;
    - clone/equality preserve the link list.

## Verification

Pass criteria:

1. `cargo test -p roastty config_link_url_finalize`
2. `cargo test -p roastty link_url_maximize_config`
3. `cargo test -p roastty config_finalize_scalar_tail`
4. `cargo test -p roastty`
5. `cargo fmt --check`
6. `git diff --check`
7. `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/106-link-url-finalize.md issues/0802-libroastty-completion-and-mac-app/README.md`

The full `cargo test -p roastty` run must pass. The existing ABI harness may
print its known enum-conversion warnings, but no new failures are acceptable.

## Design Review

Codex-native adversarial review ran in fresh context with subagent
`019eb65c-8d4e-7cb1-a96e-df9962098d8c`.

Initial verdict: **CHANGES REQUIRED**

Required finding:

- Verification did not prove the pinned upstream regex was ported exactly,
  because representative fragment checks could pass with a truncated or altered
  regex.

Fix:

- Tightened the test plan to require the default link regex to be byte-for-byte
  equal to the complete local pinned upstream URL/path regex constant.

Re-review verdict: **APPROVED**

Remaining findings: None.

## Result

**Result:** Pass

Implemented the config-owned default URL matcher and `link-url` finalization
slice.

- Added config-owned repeatable link storage using Roastty's existing
  `input::link::Link` type.
- Added the pinned upstream URL/path regex from
  `vendor/ghostty/src/config/url.zig` as local config data for the default
  matcher.
- Initialized `Config::default()` with one default URL matcher using
  `Action::Open` and `HoverMods(ctrl_or_super(Mods::new()))`.
- Added finalize-time removal of the first/default matcher when
  `link-url = false`.
- Preserved the matcher when `link-url = true`.
- Kept user `link = ...` parsing/formatting, regex compilation/matching,
  renderer link ranges, link preview UI, open-url dispatch, app C ABI exposure,
  key-remap finalization, and broader runtime link highlighting out of scope.

Verification passed:

1. `cargo test -p roastty config_link_url_finalize`
2. `cargo test -p roastty link_url_maximize_config`
3. `cargo test -p roastty config_finalize_scalar_tail`
4. `cargo test -p roastty`
5. `cargo fmt --check`
6. `git diff --check`
7. `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/106-link-url-finalize.md issues/0802-libroastty-completion-and-mac-app/README.md`

The focused link-url finalize run passed 1 test. The link-url parser/formatter
regression run passed 1 test. The scalar-finalize regression run passed 1 test.
The full `cargo test -p roastty` run passed 4590 unit tests, the ABI harness,
and doc tests. The ABI harness printed the existing 10 enum-conversion warnings.

## Conclusion

Roastty now has upstream's default URL/path matcher as config-owned data and
honors the upstream `link-url = false` finalize rule by removing that default
matcher. This closes the default-link mutation gap without claiming user
`link = ...` parsing, regex compilation, renderer link-range generation, or link
UI behavior. Remaining config-finalize gaps include key-remap finalization,
app-facing/log plumbing for finalize warnings, and broader byte-faithful config
string storage.

## Completion Review

Codex-native adversarial review ran in fresh context with subagent
`019eb664-df53-7a03-b955-fe001533d6de` after implementation and result
recording. The reviewer checked the experiment file, README, implementation diff
from the plan commit, changed source, and upstream `Config.zig`, `url.zig`,
`Link.zig`, and key-modifier sources.

Verdict: **APPROVED**

Findings: None.
