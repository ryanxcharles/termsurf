+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 783: Termio Clipboard OSC Worker Events

## Description

Experiment 782 taught the terminal parser to retain OSC 52 and Kitty clipboard
requests as typed `TerminalClipboardEvent` values. That only proves the parser
side; PTY-backed surfaces still cannot observe those events because the termio
worker currently emits only pump summaries and errors.

This experiment bridges retained terminal clipboard OSC events across the termio
worker boundary. It keeps scope deliberately narrow: surface request allocation,
frontend callbacks, and actual clipboard reads/writes remain for a later
experiment.

## Changes

- `roastty/src/termio.rs`
  - import `TerminalClipboardEvent`;
  - add `Termio::drain_clipboard_events()` as a narrow forwarding wrapper around
    `Terminal::drain_clipboard_events()`;
  - add `TermioWorkerEvent::Clipboard(TerminalClipboardEvent)`;
  - after each successful `pump_once`, drain terminal clipboard events and send
    one worker event per retained OSC request;
  - emit clipboard events before the `Pump` event from the same successful read,
    preserving the parser's retained clipboard-event order, so future request
    allocation sees the clipboard request before the coarser dirty/read summary;
  - preserve existing `TermioPump` shape so pump status tests and surface state
    handling stay stable;
  - add PTY worker tests that observe OSC 52 and Kitty clipboard `Clipboard`
    worker events, including payload/metadata/terminator retention through the
    termio boundary;
  - add an ordering test with normal output plus multiple clipboard requests in
    one read, asserting clipboard events are delivered before the corresponding
    `Pump` and in parser order.
- `roastty/src/lib.rs`
  - handle `TermioWorkerEvent::Clipboard(_)` in `Surface::apply_termio_event`
    without changing dirty, process-exited, or error state yet;
  - add a surface tick test proving queued clipboard worker events are drained
    while leaving `dirty`, `process_exited`, and `last_termio_error` unchanged.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - update the experiment index after design, result, and review;
  - leave the OSC 52 surface request allocation checklist item unchecked because
    this experiment only delivers the event to the surface boundary.

## Verification

- `cargo test -p roastty termio_clipboard -- --nocapture --test-threads=1`
- `cargo test -p roastty app_tick_drains_clipboard_termio_event -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/783-termio-clipboard-osc-worker-events.md`
- `git diff --check`

## Design Review

Codex review found three blocking design gaps:

- the worker event ordering contract was underspecified;
- Kitty clipboard coverage was missing even though Experiment 782 retained Kitty
  clipboard events too;
- the surface no-op test needed explicit assertions for `dirty`,
  `process_exited`, and `last_termio_error`.

The design now requires clipboard events to be emitted before the `Pump` from
the same successful read, preserves retained clipboard-event order, adds Kitty
coverage, and makes the surface no-op assertions concrete.

Re-review approved the revised design with no findings.

## Result

**Result:** Pass

Implemented the termio clipboard bridge without changing the existing
`TermioPump` status shape. `TermioWorkerEvent` now has a `Clipboard` variant
carrying the retained `TerminalClipboardEvent`; the worker drains events from
the terminal after each successful pump and emits those clipboard events before
the pump summary from the same read.

Surface ticking now drains clipboard worker events and intentionally leaves
surface state unchanged until a later experiment allocates frontend clipboard
requests.

Verification passed:

- `cargo test -p roastty termio_clipboard -- --nocapture --test-threads=1`
- `cargo test -p roastty app_tick_drains_clipboard_termio_event -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/783-termio-clipboard-osc-worker-events.md`
- `git diff --check`

## Conclusion

PTY-backed termio workers can now preserve OSC 52 and Kitty clipboard requests
across the worker event boundary, including ordering relative to the pump event.
The next clipboard slice can allocate surface/frontend requests from these
events instead of needing to re-parse terminal output.

## Completion Review

The first completion review found no implementation correctness issues, but
blocked result commit on missing experiment provenance frontmatter, missing
README provenance tags, and incomplete verification command recording.

Re-review approved the completed experiment with no findings after those records
were corrected.
