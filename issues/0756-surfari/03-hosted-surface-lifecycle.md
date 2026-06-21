# Experiment 3: Stress hosted WebKit surface lifecycle

## Description

Experiment 2 proved the central compositor boundary: a WebKit-owning process can
export a rendered `WKWebView` layer through a Core Animation context ID, and a
separate host process can display it with `CALayerHost` without creating its own
`WKWebView`.

This experiment turns that one-shot proof into a lifecycle stress proof. Before
Surfari or `libtermsurf_webkit` depend on this compositor path, the hosted
surface must demonstrate the behaviors Ghostboard will require in real panes:
resize, dynamic rendering, scroll, post-export navigation, repeated hide/show,
and stable shutdown.

This experiment should extend the existing `surfari-proofs/hosting-context/`
harness. It should not create Surfari, modify Ghostboard, modify
`termsurf.proto`, or patch WebKit source.

## Changes

- Extend `surfari-proofs/hosting-context/WebKitHostingProof.m` with a
  deterministic lifecycle/stress mode or deterministic owner/host timers.
- Keep the two-process architecture from Experiment 2:
  - owner process creates and owns the `WKWebView`;
  - host process creates a `CALayerHost` for the exported context ID;
  - host process does not create a `WKWebView`.
- Add explicit logs for each lifecycle milestone:
  - owner initial load finished;
  - context exported;
  - host ready;
  - JavaScript dynamic update observed;
  - scroll observed;
  - owner resize applied;
  - host resize applied;
  - post-export navigation finished;
  - host hide/show cycle 1 complete;
  - host hide/show cycle 2 complete;
  - final hosted content still visible;
  - owner and host terminate cleanly.
- If necessary, add simple IPC from owner to host so the owner can request host
  resize and hide/show cycles after the host starts.
- Add deterministic screenshots under `logs/` for the important visual states:
  dynamic update/scroll, resize, navigation, and final post-hide/show state.
- Update `surfari-proofs/hosting-context/README.md` with the stress-mode build
  and run command.
- Update this experiment's result with the exact command output, screenshot
  paths, and what each screenshot proves.
- Do not modify `webkit/src` in this experiment. If a WebKit source patch seems
  necessary, record **Partial** and design a later WebKit-branch experiment.

## Verification

Start from a clean TermSurf repo root:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
surfari-proofs/hosting-context/build.sh
```

Then run the lifecycle proof with logs in the repo `logs/` directory. The exact
command may change during implementation, but it must be recorded in the result.

The proof must demonstrate all of the following:

- The host process is separate from the owner process.
- The host process does not create a `WKWebView`.
- The hosted WebKit surface visibly updates from JavaScript after export.
- The hosted WebKit surface visibly scrolls after export.
- Owner-side WebKit resize changes are reflected in the hosted surface.
- Host-side window/layer resize does not break the hosted surface.
- Post-export navigation is visible in the hosted surface.
- At least two host hide/show cycles complete without losing the hosted surface.
- Final screenshot after hide/show still displays WebKit content in the host
  process.
- Owner and host processes terminate cleanly after the proof.
- `webkit/src` remains clean and unchanged.

**Pass** = the lifecycle harness builds, runs, logs every required milestone,
captures screenshots proving dynamic update/scroll, resize, navigation, and
final post-hide/show visibility, terminates cleanly, and leaves `webkit/src`
unchanged.

**Partial** = the harness builds and some lifecycle steps work, but one or more
required visual/lifecycle guarantees fail or require a WebKit source patch. The
result must identify the failing step and the next experiment needed.

**Fail** = the harness cannot be built or cannot reproduce the two-process
hosted WebKit surface from Experiment 2.

Before recording the result, capture:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
```

The TermSurf worktree must contain only the intended harness/docs/issue changes
plus ignored `logs/` and harness build output.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Approved.

Findings: none.
