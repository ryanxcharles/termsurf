# Experiment 6: Prove Runtime Color Scheme

## Description

Issue 816 still needs Ghostboard-specific runtime proof for color-scheme
propagation. Issue 810 classified this as `Maybe`: webtui can send
`SetColorScheme` through both the direct Roamium socket and the compositor
socket, Roamium can apply direct `SetColorScheme`, and Ghostboard creates tabs
with `dark = 0` while ignoring compositor-side `SetColorScheme` messages. The
current evidence therefore suggests post-ready webtui commands probably work
through the direct browser socket, while initial/system appearance and
compositor fallback remain ambiguous.

This experiment will add a focused Ghostboard runtime smoke for post-ready color
scheme changes. It will serve a local page that observes `prefers-color-scheme`,
drive webtui `:dark off`, `:dark on`, and `:dark system` commands, and prove
whether the page observes the expected light/dark changes under debug
Ghostboard.

Copy-current-URL is deliberately out of scope for this experiment because it is
clipboard/browser-chrome behavior, not appearance propagation.

## Changes

Planned investigation:

- Inspect the current color-scheme paths in:
  - `proto/termsurf.proto`;
  - `webtui/src/main.rs`;
  - `webtui/src/ipc.rs`;
  - `roamium/src/dispatch.rs`;
  - `ghostboard/src/apprt/termsurf.zig`;
  - Issue 810 color-scheme findings.
- Confirm whether Roamium needs a stable `set-color-scheme` trace line for the
  geometry harness. The existing direct handler applies
  `ffi::ts_set_color_scheme`, but it does not currently write a durable trace.
- Confirm whether webtui's test-only state trace should record command-level
  color-scheme events so the harness can distinguish “command accepted” from
  “page observed browser media change”.

Planned harness changes:

- Add a `color-scheme-smoke` scenario to
  `scripts/ghostboard-geometry-matrix.sh`.
- Serve a local fixture page that:
  - records the current `matchMedia("(prefers-color-scheme: dark)")` value;
  - listens for `change` events on that media query;
  - emits unique console markers and title updates for `light`, `dark`, and
    later changes;
  - uses normal page script evidence instead of screenshot color sampling as the
    primary proof.
- Launch debug Ghostboard, debug webtui, and debug Roamium using the same
  no-installed-binary guarantees as the existing Issue 816 scenarios.
- Drive webtui command mode through keyboard automation:
  - `:dark off` to establish a light baseline;
  - `:dark on` to switch to dark;
  - `:dark system` to return to webtui's current system mapping, which is
    currently implemented as light.
- Capture app log, Roamium trace, webtui state trace, and page console/title
  evidence.

Planned assertion changes:

- Roamium trace must prove each direct `SetColorScheme` message reaches the
  active browser tab with the expected boolean value.
- webtui trace must prove the user command was accepted and sent with the
  expected scheme.
- Page console/title evidence must prove Chromium's `prefers-color-scheme: dark`
  media query changes to the expected value after the command.
- The scenario must not pass from Roamium trace alone.
- The scenario must classify initial dark-state behavior separately from
  post-ready runtime command behavior. A fixed initial `dark = 0` should not
  fail this experiment unless it prevents the post-ready command sequence from
  being proven.

Planned fix policy:

- If the command is accepted by webtui but Roamium does not receive direct
  `SetColorScheme`, fix webtui direct-browser dispatch.
- If Roamium receives the message but the page does not observe a media-query
  change, fix Roamium/Chromium color-scheme application or add stronger evidence
  proving Chromium has no observable media change for that API.
- If direct runtime updates pass but Ghostboard ignores compositor
  `SetColorScheme`, record compositor fallback as a separate lower-priority
  finding rather than broadening this experiment.
- If initial/system appearance parity is wrong, record it separately with owner
  evidence and design a later experiment only if it blocks Issue 816 parity.

Planned issue-doc changes:

- Record the fixture, command sequence, logs, traces, and owner classification.
- Record whether remaining color-scheme work is initial/system appearance,
  compositor fallback, or no further Issue 816 work.
- Leave copy-current-URL as the next Issue 816 gap.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0816-ghostboard-browser-state-interruptions/README.md issues/0816-ghostboard-browser-state-interruptions/06-prove-runtime-color-scheme.md`.
2. If Rust files change, `cargo fmt -- <changed-rust-files>`.
3. If Zig files change, `zig fmt <changed-zig-files>`.

Static/build checks:

1. `prettier --check --prose-wrap always --print-width 80 issues/0816-ghostboard-browser-state-interruptions/README.md issues/0816-ghostboard-browser-state-interruptions/06-prove-runtime-color-scheme.md`.
2. `bash -n scripts/ghostboard-geometry-matrix.sh`.
3. `cargo check -p webtui` if webtui changes.
4. `cargo build -p webtui` if webtui changes.
5. `cargo check -p roamium` if Roamium changes.
6. `./scripts/build.sh roamium` if Roamium changes.
7. If Ghostboard Zig or non-`macos/` Ghostboard files change, run
   `cd ghostboard && zig build -Demit-macos-app=false`.
8. If Ghostboard app files change or a Ghostboard rebuild is needed, run
   `cd ghostboard && macos/build.nu --configuration Debug --action build`.
9. `shellcheck scripts/ghostboard-geometry-matrix.sh` if available.
10. `git diff --check`.

Design gate:

- This experiment file is plan-only until a fresh-context design review approves
  it.
- Record design review findings and fixes in this file.
- Commit the approved experiment plan before implementation begins.

Completion gate:

- After implementation and verification, record `## Result` and `## Conclusion`
  in this file.
- Update the Issue 816 README experiment status from `Designed` to the final
  result.
- Request a fresh-context completion review, fix all real findings, and record
  the final completion-review verdict in this file.
- Commit the reviewed experiment result separately before designing or
  implementing the next experiment.

Runtime checks:

1. `scripts/ghostboard-geometry-matrix.sh color-scheme-smoke`.
2. Confirm the initial local page loads and reports its ready marker.
3. Confirm `:dark off` is accepted by webtui, reaches Roamium as
   `SetColorScheme dark=false`, and the page reports light mode.
4. Confirm `:dark on` is accepted by webtui, reaches Roamium as
   `SetColorScheme dark=true`, and the page reports dark mode.
5. Confirm `:dark system` is accepted by webtui, reaches Roamium as
   `SetColorScheme dark=false` with the current implementation, and the page
   reports light mode again.
6. Confirm the active browser tab and pane IDs remain the same across all
   commands.

Pass criteria:

- The scenario runs to completion under debug Ghostboard without installed
  binary leakage.
- Each runtime color-scheme command has durable webtui command evidence, Roamium
  direct-message evidence, and page-level media-query evidence.
- The scenario distinguishes post-ready runtime color changes from initial
  tab-create dark-state and compositor fallback behavior.
- Any app code change is owned by the component proven responsible and is no
  broader than needed.

Partial criteria:

- webtui and Roamium traces prove the command path, but page media-query
  evidence is unavailable or ambiguous.
- Runtime direct color changes pass, but initial/system appearance or compositor
  fallback remains unproven and is recorded separately.
- The owner is proven, but a Chromium or Ghostboard fix is too broad for this
  experiment.

Fail criteria:

- The harness cannot distinguish webtui command acceptance, Roamium delivery,
  and page-observed media-query changes.
- The scenario passes only from Roamium trace or app logs without page-level
  evidence.
- The implementation hides a color-scheme failure by weakening assertions,
  skipping one command direction, or treating initial state as proof of runtime
  updates.

## Design Review

Fresh-context adversarial design review by Codex subagent `Lagrange`:

- **Initial verdict:** Changes required.
- **Required finding:** The original design included the design-review and
  plan-commit gate, but did not explicitly include the completion-review and
  result-commit gate required by the issue workflow.
- **Resolution:** Accepted. The design now includes a completion gate requiring
  result/conclusion recording, README status update, fresh-context completion
  review, fixes for real findings, final review verdict recording, and a
  separate result commit before moving to the next experiment.
- **Re-review verdict:** Approved. The reviewer confirmed the completion gate is
  now present and no required findings remain.
