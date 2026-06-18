# Experiment 2: Fix Initial Loading-State Start

## Description

Experiment 1 proved the normal Ghostboard/webtui direct browser-state path, but
found one concrete gap: the initial fixture navigation reached webtui as
`progress`/`done` without a literal `loading` state, while a later Browse-mode
Cmd-R reload did emit `loading` before `progress`/`done`.

This experiment will identify the owner of that initial-load difference and fix
the smallest component that is actually responsible. Static evidence shows
`TsTabObserver::DidStartLoading()` sends `loading`, so the likely failure mode
is ordering: the initial navigation may begin before the observer/callback path
is ready. That is a hypothesis only; the experiment must prove ownership from
runtime evidence before changing code.

## Changes

Planned investigation:

- Inspect the initial navigation path through Roamium and libtermsurf Chromium:
  `roamium/src/dispatch.rs`,
  `chromium/src/content/libtermsurf_chromium/ts_tab_observer.cc`, and the
  WebContents creation/navigation path in
  `chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.cc`.
- Run the existing `browser-state-smoke` scenario and compare:
  - webtui state trace;
  - Roamium trace;
  - Ghostboard app log;
  - Chromium/libtermsurf code order for observer registration and initial
    navigation.
- Add temporary or test-gated callback-level tracing before ownership
  classification:
  - first in Roamium's `on_loading_state` callback, recording raw state/progress
    before protobuf serialization;
  - if Roamium never observes the initial `loading` callback, add the narrowest
    libtermsurf/Chromium notification trace needed to distinguish
    `TsNotifyLoadingState("loading", 0)` not firing from Roamium dropping it.
- Classify ownership:
  - **Chromium/libtermsurf** if Roamium never receives `loading` for the initial
    load and Chromium/libtermsurf trace shows the notification did not fire;
  - **Roamium** if the C callback fires but Roamium fails to serialize/send it;
  - **webtui** only if Roamium sends a literal `loading` protobuf but webtui
    fails to record/render that literal event;
  - **harness** if the fixture or timing masks a valid event.

Planned fix:

- If the owner is webtui, fix webtui so it records/renders the literal `loading`
  event it already receives. Do not pass this experiment by deriving loading
  from `progress` or weakening the literal-event assertion.
- If the owner is Roamium, fix the callback dispatch/serialization path and add
  trace evidence.
- If the owner is Chromium/libtermsurf, create a new Chromium issue branch as
  required by `AGENTS.md`, then fix the initial navigation ordering or emit a
  deterministic initial `loading` notification at the engine boundary before the
  first `progress`.
- Update `scripts/ghostboard-geometry-matrix.sh` so `browser-state-smoke`
  requires the initial fixture load to include a literal `state=loading` instead
  of recording a Partial.

Planned issue-doc changes:

- Record the ownership evidence, exact code owner, changed files, verification
  commands, and the before/after trace lines.
- Update Experiment 1's follow-up finding only by reference from this
  experiment; do not rewrite the historical Experiment 1 result.

## Verification

Formatting actions:

1. `prettier --write --prose-wrap always --print-width 80 issues/0816-ghostboard-browser-state-interruptions/README.md issues/0816-ghostboard-browser-state-interruptions/02-fix-initial-loading-state-start.md`.
2. If Rust files change, `cargo fmt -- <changed-rust-files>`.
3. If Zig files change, `zig fmt <changed-zig-files>`.

Static/build checks:

1. `prettier --check --prose-wrap always --print-width 80 issues/0816-ghostboard-browser-state-interruptions/README.md issues/0816-ghostboard-browser-state-interruptions/02-fix-initial-loading-state-start.md`.
2. `bash -n scripts/ghostboard-geometry-matrix.sh`.
3. `cargo check -p webtui` if webtui changes.
4. `cargo build -p webtui` if webtui changes.
5. If Roamium changes, run `./scripts/build.sh roamium` and
   `cargo check -p roamium` if the workspace package is available.
6. If Chromium changes, run the narrow Chromium/Roamium build needed to
   regenerate `chromium/src/out/Default/roamium`.
7. `shellcheck scripts/ghostboard-geometry-matrix.sh` if available.
8. `git diff --check`.

Runtime checks:

1. `scripts/ghostboard-geometry-matrix.sh browser-state-smoke`.
2. Confirm the webtui state trace now records an initial-load sequence with
   `state=loading`, then `state=progress`, then `state=done` for the fixture
   URL.
3. Confirm reload still records `state=loading`, `progress`, and `done`.
4. Confirm the previously passing Experiment 1 assertions still pass: URL,
   title, console, hover target, white background, reload marker, and fresh
   post-click `_blank` URL/title evidence.

Pass criteria:

- The initial fixture load emits a literal `loading` state at the webtui
  consumer boundary before `done`.
- The fix is owned by the proven responsible component and is no broader than
  needed.
- `browser-state-smoke` exits successfully without a Partial loading-start note.
- Existing URL/title/console/hover/reload/target-blank/white-background
  assertions still pass.

Partial criteria:

- The owner is proven, but the fix requires a broader Chromium rebuild or branch
  step that cannot be completed in this experiment.
- A fix works in trace evidence but the durable harness assertion remains too
  flaky for a result commit.

Fail criteria:

- Ownership cannot be distinguished from available traces and code inspection.
- The candidate fix changes unrelated browser-state behavior or hides the
  missing initial `loading` event by weakening the assertion.

## Design Review

Fresh-context adversarial review by Codex subagent `Confucius`:

- **Initial verdict:** Changes required.
- **Required finding:** The original webtui fallback could have passed by
  treating `progress` as visible loading while the experiment still claimed to
  prove a literal `state=loading` event.
- **Required finding:** The original ownership proof relied on traces that could
  not distinguish Chromium callback absence from Roamium callback/drop behavior.
- **Optional finding:** The Roamium verification command was vague.
- **Resolution:** Accepted all findings. The design now forbids passing by
  deriving loading from `progress`, requires callback-level Roamium
  `on_loading_state` tracing before ownership classification, escalates to
  narrow libtermsurf/Chromium notification tracing if needed, and names
  `./scripts/build.sh roamium` plus `cargo check -p roamium` when applicable.
- **Re-review verdict:** Approved. The reviewer confirmed the prior findings
  were resolved and no new required findings were introduced.
