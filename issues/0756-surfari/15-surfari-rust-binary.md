# Experiment 15: Stand up the Surfari Rust binary

## Description

Experiments 5-14 built out the macOS `libtermsurf_webkit` C ABI and proved it
with a deterministic C smoke harness. Issue 756 still needs the actual Surfari
Rust browser process: a Roamium-style binary that links `libtermsurf_webkit`,
speaks the TermSurf protobuf/Unix-socket protocol, and drives WebKit through the
C ABI.

This experiment should create the initial buildable `surfari` Rust crate and
binary. It should reuse Roamium's proven Rust structure as directly as possible:
`main.rs`, `ffi.rs`, `dispatch.rs`, `ipc.rs`, `proto.rs`, and a `build.rs` that
compiles `termsurf.proto`. The key difference is link target and runtime label:
Surfari links `libtermsurf_webkit.dylib` from
`surfari/libtermsurf_webkit/build`, not Chromium's `libtermsurf_chromium.dylib`.

This experiment should not integrate Surfari into Ghostboard, add browser
selection/configuration, update Homebrew/release scripts, implement DevTools,
modify `termsurf.proto`, or add a full fake-GUI IPC harness. Those are follow-up
experiments. The goal here is a clean, reproducible Rust binary build that can
be used by the next experiment.

## Changes

- Add a `surfari` Rust crate:
  - `surfari/Cargo.toml`;
  - `surfari/build.rs`;
  - `surfari/src/main.rs`;
  - `surfari/src/ffi.rs`;
  - `surfari/src/dispatch.rs`;
  - `surfari/src/ipc.rs`;
  - `surfari/src/proto.rs`.
- Add `surfari` to the root Cargo workspace members.
- Reuse Roamium's IPC/protobuf/dispatch structure unless a direct name or link
  target must change.
- Adjust Rust diagnostics and trace prefixes from `Roamium` / `roamium` to
  `Surfari` / `surfari`, so logs distinguish the WebKit process from Chromium.
- Make `surfari/build.rs`:
  - require `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib`;
  - link `termsurf_webkit`;
  - add native link search path for `surfari/libtermsurf_webkit/build`;
  - add rpaths for `@loader_path/.` and the local WebKit C ABI build directory;
  - compile `../proto/termsurf.proto` with `prost-build`, matching Roamium.
- Keep the FFI signatures aligned with
  `surfari/libtermsurf_webkit/include/libtermsurf_webkit.h`.
- Keep `CreateDevtoolsTab` wired through `ts_create_devtools_web_contents` even
  though the current C ABI returns `nullptr`; DevTools remains explicitly
  unsupported and should not be solved in this experiment.
- Avoid modifying `roamium/`, `webtui/`, `ghostboard/`, `termsurf.proto`, or
  WebKit source.

## Verification

Start from a clean repo root:

```bash
git status --short
git -C webkit/src status --short
```

Build the WebKit C ABI first, then build Surfari:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
```

Verify the binary and linkage:

```bash
test -x target/debug/surfari
otool -L target/debug/surfari | rg 'libtermsurf_webkit|WebKit|JavaScriptCore'
nm -gU target/debug/surfari | rg 'ts_content_main|ts_set_on_initialized|ts_set_on_renderer_crashed'
```

Run focused source checks:

```bash
cargo fmt -p surfari -- --check
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/15-surfari-rust-binary.md
rg -n 'Roamium|roamium|termsurf_chromium|chromium/src/out/Default' surfari \
  -g '*.rs' -g 'Cargo.toml' -g 'build.rs'
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

The `rg` command should return no Rust/build references to Roamium or Chromium.
It may still find historical issue text or C smoke-test fixture strings outside
the `*.rs`, `Cargo.toml`, and `build.rs` scope, which are not part of this
experiment.

**Pass** = `cargo build -p surfari` produces `target/debug/surfari`, the binary
links to `libtermsurf_webkit`, Surfari Rust sources no longer carry Roamium or
Chromium link/build names, formatting and whitespace checks pass, and
`webkit/src` remains unchanged.

**Partial** = the crate builds only with manual environment variables or with a
temporary limitation that must be fixed before the fake-GUI IPC experiment can
run. Record the exact limitation and whether the next experiment should fix it.

**Fail** = the crate does not build, links Chromium instead of WebKit, requires
protocol/Ghostboard/WebKit source changes, or cannot produce a concrete next
step.

## Design Review

Adversarial design review approved the experiment with no findings. The reviewer
inspected the workflow contract, Issue 756 README, this experiment design,
Roamium's Rust/build structure, and the current `libtermsurf_webkit` header and
README. The reviewer confirmed the scope is narrow, the README links Experiment
15 as `Designed`, the planned linking strategy is coherent, the verification
commands cover the intended proof, and no Ghostboard/protocol/WebKit source
changes are included in the plan.
