# Experiment 139: Image Storage Limit Runtime

## Description

`RUNTIME-009B2B2B3B2B2B2B` still groups other remaining terminal behavior
effects. One concrete unproven config-driven terminal effect in that gap is
`image-storage-limit`, which controls the kitty graphics image storage quota.

Pinned Ghostty stores `image-storage-limit` in `termio.DerivedConfig`, passes it
into `Terminal.init` as `kitty_image_storage_limit`, and applies live config
updates in `Termio.changeConfig`:

- startup: `.kitty_image_storage_limit = opts.config.image_storage_limit`;
- live update:
  `self.terminal.setKittyGraphicsSizeLimit(..., config.image_storage_limit)`;
- live update also restores kitty image loading limits to `.all`.

Roastty parses and formats `image-storage-limit`, and the terminal core already
has direct setters/tests for kitty image storage limits. However,
`TermioSpawnOptions` does not currently carry parsed
`Config.image_storage_limit` from app/surface startup, and
`Surface::apply_config` does not update the active terminal's kitty image
storage quota. This leaves the PTY-backed runtime path unproven and likely
hard-coded to the default limit.

This experiment will split the remaining terminal row:

- `RUNTIME-009B2B2B3B2B2B2B1`: **Oracle complete** for `image-storage-limit`
  kitty graphics storage quota startup and live update effects.
- `RUNTIME-009B2B2B3B2B2B2B2`: **Gap** for other remaining terminal behavior
  effects.

This experiment will not claim full kitty graphics protocol parity, image
rendering pixel parity, medium loading policies beyond Ghostty's config-driven
`.all` reset, or renderer-visible image output.

## Changes

- `roastty/src/termio.rs`
  - Add `image_storage_limit` to `TermioSpawnOptions`.
  - Pass it into the initialized terminal's kitty image storage limit.
  - Add a focused Termio test proving non-default spawn options reach the
    PTY-backed terminal runtime.
- `roastty/src/lib.rs`
  - Thread parsed `Config.image_storage_limit` into initial surface Termio spawn
    options.
  - Update existing active surfaces in `Surface::apply_config` so live app
    config updates refresh the terminal's kitty image storage limit.
  - Add a focused surface/app config test proving startup and live update
    propagation.
- `roastty/src/terminal/terminal.rs`
  - Reuse the existing terminal kitty image storage limit setter and direct
    terminal tests where possible.
  - Add a small focused test only if needed to prove reset/update behavior not
    already covered by existing kitty graphics storage tests.
- `issues/0805-roastty-ghostty-parity/image_storage_limit_runtime_parity.py`
  - Add a static guard checking pinned Ghostty markers: `image_storage_limit`,
    `config.@"image-storage-limit"`,
    `.kitty_image_storage_limit = opts.config.image_storage_limit`,
    `setKittyGraphicsSizeLimit`, and `setKittyGraphicsLoadingLimits(.all)`.
  - Check Roastty parser/runtime/update markers, focused tests, and the
    inventory split.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-009B2B2B3B2B2B2B` into an image-storage-limit complete row
    and a reduced remaining-terminal gap row.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223 runtime static guards that hard-code current runtime row
  counts or the remaining terminal gap row
  - Update expected counts after the split.
  - Update references from `RUNTIME-009B2B2B3B2B2B2B` to the reduced remaining
    terminal gap row.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Pinned Ghostty evidence shows `image-storage-limit` reaches terminal startup
  kitty image storage limit and live `changeConfig` updates.
- Roastty `TermioSpawnOptions` carries parsed `image-storage-limit` into the
  initialized terminal.
- Roastty active surfaces update the terminal kitty image storage quota when app
  config changes.
- Live updates preserve Ghostty's config-driven behavior of restoring kitty
  image loading limits to all enabled media.
- `RUNTIME-009B2B2B3B2B2B2B1` is Oracle complete and cites Termio, surface, and
  static guard evidence.
- `RUNTIME-009B2B2B3B2B2B2B2` remains `Gap` for other remaining terminal
  behavior effects.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml termio_image_storage_limit_runtime
cargo test --manifest-path roastty/Cargo.toml surface_image_storage_limit_runtime
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/image_storage_limit_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/139-image-storage-limit-runtime.md
git diff --check
```

Fail criteria:

- The experiment only proves direct terminal setters and not parsed config
  startup/live update paths.
- Live config update changes stored app config but not the active terminal kitty
  image storage limit.
- The implementation claims broader kitty graphics protocol or renderer image
  output parity.
- The reduced remaining terminal gap is removed or CFG-223 is marked complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer found no findings.

## Result

**Result:** Pass.

Roastty now threads parsed `image-storage-limit` into PTY-backed terminal
startup and live app config updates. `TermioSpawnOptions` carries the configured
limit into the initialized terminal, and `Surface::apply_config` updates the
active terminal's kitty image storage quota when app config changes.

The implementation also mirrors pinned Ghostty's live config behavior of
restoring kitty image loading limits to all enabled media when
`image-storage-limit` is applied.

The CFG-223 inventory now splits `RUNTIME-009B2B2B3B2B2B2B` into:

- `RUNTIME-009B2B2B3B2B2B2B1`: **Oracle complete** for `image-storage-limit`
  kitty graphics storage quota startup and live update effects.
- `RUNTIME-009B2B2B3B2B2B2B2`: **Gap** for other remaining terminal behavior
  effects.

Verification passed:

```bash
cargo fmt --manifest-path roastty/Cargo.toml
cargo test --manifest-path roastty/Cargo.toml termio_image_storage_limit_runtime
cargo test --manifest-path roastty/Cargo.toml surface_image_storage_limit_runtime
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/image_storage_limit_runtime_parity.py
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$f" >/tmp/$(basename "$f").out || { echo FAIL:$f; cat /tmp/$(basename "$f").out; exit 1; }; done; echo all_runtime_parity_guards=pass
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

The regenerated inventory reported:

```text
runtime_rows=48
oracle_complete=41
closed=43
audit_covered=0
incomplete=5
gap=5
cfg223=Gap
```

## Conclusion

`image-storage-limit` is config-derived terminal runtime state in pinned
Ghostty. Roastty now applies the parsed limit at PTY-backed terminal startup and
live config update, while resetting kitty image loading media to all enabled on
live update. This closes the storage-quota runtime effect only; kitty graphics
protocol behavior and renderer-visible image output remain separate parity
surfaces.

## Completion Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer found no findings and confirmed the result commit had not yet been
made. The reviewer specifically confirmed that `HEAD` was still
`0a2a4df47 Plan image limit parity` with result changes uncommitted.
