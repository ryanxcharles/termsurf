+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 550: thread QoS class (os::macos)

## Description

Continuing the `os` module (Experiments 541–549), this experiment opens
`os::macos` with the **thread Quality-of-Service class** helpers from upstream
`os/macos.zig`: the `QosClass` enum and `set_qos_class`, which set the running
thread's QoS via `pthread_set_qos_class_self_np`. roastty will use this to tune
its render / IO threads (e.g. `user_interactive` for the renderer). This is the
self-contained, libc-only part of `macos.zig`; the objc-based version check and
the bundle-id directory helpers defer (see Deferred).

## Upstream behavior

`os/macos.zig`:

```zig
/// QoS classes (the macOS thread quality-of-service levels).
pub const QosClass = enum(c_uint) {
    user_interactive = 0x21,
    user_initiated = 0x19,
    default = 0x15,
    utility = 0x11,
    background = 0x09,
    unspecified = 0x00,
};

extern "c" fn pthread_set_qos_class_self_np(qos_class: QosClass, relative_priority: c_int) c_int;

pub const SetQosClassError = error{ ThreadIncompatible };

/// Set the QoS class of the running thread.
pub fn setQosClass(class: QosClass) !void {
    return switch (std.posix.errno(pthread_set_qos_class_self_np(class, 0))) {
        .SUCCESS => {},
        .PERM => error.ThreadIncompatible,
        // EPERM is the only known error per the man page.
        else => @panic("unexpected pthread_set_qos_class_self_np error"),
    };
}
```

- `QosClass` is the set of macOS QoS levels with their exact `qos_class_t`
  values (`user_interactive` `0x21` … `unspecified` `0x00`).
- `setQosClass(class)` calls `pthread_set_qos_class_self_np(class, 0)` for the
  current thread. Success ⇒ ok; `EPERM` ⇒ `ThreadIncompatible` (the thread can't
  have its QoS changed, usually because a different pthread API made it an
  invalid target); any other errno is unexpected and panics.

## Rust mapping (`roastty/src/os/macos.rs`)

`libc` already exposes `qos_class_t` (with the same values) and
`pthread_set_qos_class_self_np`, so no `extern` block is needed. `QosClass` is a
`#[repr(u32)]` enum mirroring upstream, mapped to `libc::qos_class_t`:

```rust
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

/// Map a `pthread_set_qos_class_self_np` return code to a result. The function returns
/// **zero on success, otherwise an errno value directly** (per the Apple `<pthread/qos.h>`
/// docs) — it does *not* use the `-1`/`errno` convention, so the code is matched directly.
fn map_qos_result(rc: libc::c_int) -> Result<(), SetQosClassError> {
    match rc {
        0 => Ok(()),
        // EPERM is the only known error per the man page.
        libc::EPERM => Err(SetQosClassError::ThreadIncompatible),
        _ => panic!("unexpected pthread_set_qos_class_self_np error"),
    }
}
```

`QosClass` keeps the exact upstream discriminants; `to_libc` maps each to the
matching `libc::qos_class_t` (same values). `set_qos_class` calls the libc
function for the current thread; `map_qos_result` maps the result: `0` ⇒ `Ok`;
`EPERM` ⇒ `ThreadIncompatible`; any other code ⇒ panic — realizing upstream's
documented intent (`.SUCCESS` / `.PERM` / `else`). Note:
`pthread_set_qos_class_self_np` returns the errno value _directly_ (Apple's
`<pthread/qos.h>`: "Zero if successful, otherwise an errno value"), so the code
matches `rc` itself rather than reading `errno` — extracting `map_qos_result`
also makes the `EPERM` arm unit-testable without forcing the live thread into an
incompatible state.

## Scope / faithfulness notes

- **Ported (bridged)**: `os.macos.QosClass` → `os::macos::QosClass`;
  `os.macos.SetQosClassError` → `os::macos::SetQosClassError`;
  `os.macos.setQosClass` → `os::macos::set_qos_class`.
- **Faithful**: the QoS levels with their exact values (`0x21` / `0x19` / `0x15`
  / `0x11` / `0x09` / `0x00`); `set_qos_class` setting the current thread's QoS
  and mapping success / `EPERM` (`ThreadIncompatible`) / other-errno (panic).
- **Faithful adaptation**: the upstream `extern` `pthread_set_qos_class_self_np`
  / `QosClass(c_uint)` → `libc::pthread_set_qos_class_self_np` /
  `libc::qos_class_t` (same values; `to_libc` maps the Rust enum); the `errno`
  switch → a direct `rc` match in `map_qos_result` (the function returns the
  errno value directly per Apple's `<pthread/qos.h>`, so no `errno` read — this
  realizes upstream's documented `.SUCCESS` / `.PERM` / `else` intent); `!void`
  → `Result<(), SetQosClassError>`.
- **Deferred**: `isAtLeastVersion` (objc `NSProcessInfo`); `appSupportDir` /
  `cacheDir` and their `commonDir` / `NSSearchPath*` helpers (objc
  `NSFileManager` + `build_config.bundle_id` — blocked on roastty's
  product-naming decision, like `loadDefaultFiles`); `pthread_setname_np` (a
  separate thread-naming concern); `NSOperatingSystemVersion`.
- No C ABI/header/ABI-inventory change (internal Rust). New `os::macos` module.

## Changes

1. `roastty/src/os/macos.rs` (new): `QosClass` (+ `to_libc`),
   `SetQosClassError`, `set_qos_class`, `map_qos_result`.
2. `roastty/src/os/mod.rs`: add `pub(crate) mod macos;`.
3. Tests (in `macos.rs`):
   - **discriminants**: each `QosClass as u32` equals its upstream value
     (`UserInteractive` `0x21`, `UserInitiated` `0x19`, `Default` `0x15`,
     `Utility` `0x11`, `Background` `0x09`, `Unspecified` `0x00`), and
     `to_libc(class) as u32` equals the same value.
   - **result mapping**: `map_qos_result(0) == Ok(())`;
     `map_qos_result(libc::EPERM) == Err(ThreadIncompatible)`; a
     `#[should_panic]` test that `map_qos_result(libc::EINVAL)` panics (the
     unexpected-errno arm).
   - **set succeeds on a normal thread**: `set_qos_class(QosClass::Default)`
     returns `Ok(())` (a cargo test thread is a normal pthread; setting its QoS
     is benign).
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty os::macos
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/os/macos.rs roastty/src/os/mod.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `QosClass` has the exact upstream values and `set_qos_class` sets the current
  thread's QoS, mapping `EPERM` ⇒ `ThreadIncompatible` and other errno ⇒ panic —
  faithful to `os/macos.zig`;
- the tests pass (discriminants + set succeeds), and the existing tests still
  pass;
- the objc version check and bundle-id directory helpers stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a QoS value or the `set_qos_class` error mapping
diverges from upstream, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex's first design review raised **one Required** finding (and an Optional),
both now fixed; the corrected design was **re-reviewed and approved with no
findings**.

- **errno convention (Required, fixed)**: the design read `last_os_error()`
  after a non-zero return, but Apple's `<pthread/qos.h>` documents
  `pthread_set_qos_class_self_np` as returning "Zero if successful, otherwise an
  errno value" **directly** (not the `-1`/`errno` convention). Fixed by matching
  on `rc` itself in `map_qos_result` (`0` ⇒ `Ok`, `EPERM` ⇒
  `ThreadIncompatible`, else panic) — the intent-faithful behavior. (Upstream's
  `std.posix.errno(rc)` only reads `errno` when `rc == -1`, so matching `rc`
  directly realizes upstream's documented `.SUCCESS` / `.PERM` / `else` intent.)
- **(Optional, addressed)**: extracted `map_qos_result(rc)` so the `EPERM` and
  unexpected- errno arms are unit-testable without forcing the live thread into
  an incompatible state.

On re-review Codex confirmed the direct `rc` match is correct, the extracted
mapper resolves the testability concern, and the remaining choices are sound
(exact QoS values, explicit `to_libc` mapping, the benign
`set_qos_class(Default)` smoke test, and deferring the objc/bundle-dir helpers).

Review artifacts:

- Prompt: `logs/codex-review/20260604-d550-prompt.md` (design),
  `logs/codex-review/20260604-d550b-prompt.md` (design re-review)
- Result: `logs/codex-review/20260604-d550-last-message.md` (design),
  `logs/codex-review/20260604-d550b-last-message.md` (design re-review)
