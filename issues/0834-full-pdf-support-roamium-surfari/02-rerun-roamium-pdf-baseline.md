# Experiment 2: Rerun the Roamium PDF Baseline

## Description

Experiment 1 found that Roamium has strong historical PDF evidence, but the
specific log directories from the prior PDF issues are not present in this
checkout. Before adding new PDF features or starting Surfari PDF work, we need
fresh current-tree proof for the existing Roamium baseline.

This experiment reruns the existing Roamium PDF probes without changing product
behavior. The goal is to convert current `Weak evidence` matrix rows to `Proven`
only where current logs directly support that status, and to identify the first
failing layer if any baseline regresses.

## Changes

1. Prepare a clean baseline run.

   Capture the current state before running probes:

   ```bash
   git status --short
   git -C chromium/src status --short
   git -C chromium/src rev-parse --abbrev-ref HEAD
   git -C chromium/src rev-parse HEAD
   ```

   Do not modify Chromium, Roamium, Ghostboard, Surfari, WebKit, protocol,
   fixtures, or harness behavior in this experiment unless a baseline probe
   cannot run because of an obvious harness environment issue. Any such change
   must be narrowly documented and reviewed before being included in the result.

2. Run current Roamium PDF baseline probes.

   Use fresh log directories under `logs/issue-834-exp2-*`:

   ```bash
   python3 scripts/test-issue-794-pdf-toolbar.py \
     --log-dir logs/issue-834-exp2-save-title-local \
     --serve-bitcoin-pdf \
     --probe save-print-title-local \
     --enable-pdf-print-intercept
   python3 scripts/test-issue-794-pdf-toolbar.py \
     --log-dir logs/issue-834-exp2-toolbar-events \
     --serve-bitcoin-pdf \
     --probe events
   python3 scripts/test-issue-794-protocol-scroll.py \
     --log-dir logs/issue-834-exp2-protocol-scroll \
     --serve-bitcoin-pdf
   python3 scripts/test-issue-794-protocol-resize.py \
     --log-dir logs/issue-834-exp2-protocol-resize \
     --serve-bitcoin-pdf
   python3 scripts/test-issue-794-protocol-mouse.py \
     --log-dir logs/issue-834-exp2-protocol-mouse-click \
     --serve-bitcoin-pdf \
     --action click
   python3 scripts/test-issue-794-protocol-mouse.py \
     --log-dir logs/issue-834-exp2-protocol-select-copy \
     --serve-bitcoin-pdf \
     --action key-select-copy
   python3 scripts/test-issue-796-pdf-security.py \
     --log-dir logs/issue-834-exp2-security
   ```

   Also run one current non-PDF Roamium smoke using the existing interaction
   fixture:

   ```bash
   mkdir -p logs/issue-834-exp2-non-pdf-html
   python3 -m http.server 9791 \
     --bind 127.0.0.1 \
     --directory test-html/public \
     > logs/issue-834-exp2-non-pdf-html/http-server.log 2>&1 &
   HTML_SERVER_PID=$!
   python3 scripts/test-issue-794-protocol-mouse.py \
     http://127.0.0.1:9791/test-interactions.html \
     --log-dir logs/issue-834-exp2-non-pdf-html \
     --action click \
     --url-contains test-interactions.html
   kill "$HTML_SERVER_PID"
   ```

3. Record probe summaries.

   For each run, record:

   - command;
   - exit status;
   - log directory;
   - summary file path;
   - summary status or `first_failing_hop`;
   - which matrix rows the run proves or fails to prove.

4. Update the Experiment 1 matrix conservatively.

   In Experiment 2's result, list matrix status updates driven by the fresh
   runs. Do not edit Experiment 1's historical matrix in place; instead, record
   the current baseline delta in this experiment and update the Issue 834 README
   checklist only when the Roamium baseline is genuinely current.

5. Stop on baseline failure.

   If any baseline probe fails, do not continue into new feature work. Record
   the first failing layer, classify the result as `Partial` or `Fail`, and
   recommend the next experiment around that failure.

## Verification

Verification for the completed result is:

- all intended baseline probe commands are run, or skipped only with a concrete
  blocker recorded;
- every produced summary file is cited;
- passing rows include concrete evidence, not just zero exit status;
- failing rows record the first failing layer when available;
- no product code changes are made unless explicitly justified as a harness
  environment fix;
- no Chromium source changes are made unless a fresh Chromium branch and patch
  archive are created according to `chromium/AGENTS.md`;
- native print is only exercised through the contained intercept path and no
  real print job is submitted;
- README experiment status is updated from `Designed` to the final result;
- completion review is recorded before the result commit;
- markdown is formatted with Prettier;
- `git diff --check` passes.

## Design Review

Fresh-context adversarial review by Codex subagent `Laplace`: **Changes
required**, then **Approved** after fixes.

Required finding:

- The initial design omitted the non-PDF baseline smoke required by Experiment
  1's matrix and fast-smoke tier.

Fix:

- Added a concrete non-PDF Roamium smoke using
  `test-html/public/test-interactions.html`, a local `python3 -m http.server`,
  and `scripts/test-issue-794-protocol-mouse.py --action click`.

Re-review verdict: **Approved**.

## Pass Criteria

This experiment passes if the current Roamium baseline probes pass and produce
fresh evidence for the already-working core PDF rows: rendering, embedded/local
parity, scroll, resize, click, selection/copy, toolbar zoom/fit/rotate,
save/download, title propagation, contained print safety, security, and non-PDF
regression where covered by the selected probes.

## Partial Criteria

This experiment is partial if at least one baseline area still works and is
recorded with fresh evidence, but one or more baseline probes fail or cannot
run.

## Failure Criteria

This experiment fails if the baseline cannot be run at all, if it changes
product behavior instead of measuring the current baseline, if it clicks
production native print without containment, or if it claims current proof
without fresh probe evidence.

## Result

**Result:** Pass

The current Roamium PDF baseline was rerun without product-code changes. The
main repo and Chromium checkout were clean before the probes. Chromium was on
branch `148.0.7778.97-issue-799-exp13` at
`7d0ed56bf1693468a6e992932c32fdf67f29ceaf`, and the tested Roamium binary was
`chromium/src/out/Default/roamium`.

| Area                                                     | Command                                                                                                                                                                                                                                                                      | Exit status                                     | Summary result                                                                                                | Evidence                                                                                                                                                           |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| State capture                                            | `git status --short`; `git -C chromium/src status --short`; `git -C chromium/src rev-parse --abbrev-ref HEAD`; `git -C chromium/src rev-parse HEAD`                                                                                                                          | 0                                               | Clean state captured                                                                                          | `logs/issue-834-exp2-state.txt`                                                                                                                                    |
| Save, title, local parity, embedded PDF, contained print | `python3 scripts/test-issue-794-pdf-toolbar.py --log-dir logs/issue-834-exp2-save-title-local --serve-bitcoin-pdf --probe save-print-title-local --enable-pdf-print-intercept`                                                                                               | 0                                               | `status = "pass"`; `first_failing_hop = "no-failure-observed"`                                                | `logs/issue-834-exp2-save-title-local/pdf-toolbar-summary.json`; `logs/issue-834-exp2-save-title-local/save-print-title-local/save-print-title-local-summary.json` |
| Toolbar events                                           | `python3 scripts/test-issue-794-pdf-toolbar.py --log-dir logs/issue-834-exp2-toolbar-events --serve-bitcoin-pdf --probe events`                                                                                                                                              | 0                                               | `toolbar_probe_status = "ok"`; `toolbar_summary.status = "pass"`; `first_failing_hop = "no-failure-observed"` | `logs/issue-834-exp2-toolbar-events/pdf-toolbar-summary.json`; `logs/issue-834-exp2-toolbar-events/toolbar-events/toolbar-events-summary.json`                     |
| Protocol scroll                                          | `python3 scripts/test-issue-794-protocol-scroll.py --log-dir logs/issue-834-exp2-protocol-scroll --serve-bitcoin-pdf`                                                                                                                                                        | 0                                               | `first_failing_hop = "no-failure-observed"`                                                                   | `logs/issue-834-exp2-protocol-scroll/protocol-scroll-summary.json`                                                                                                 |
| Protocol resize                                          | `python3 scripts/test-issue-794-protocol-resize.py --log-dir logs/issue-834-exp2-protocol-resize --serve-bitcoin-pdf`                                                                                                                                                        | 0                                               | `first_failing_hop = "no-failure-observed"`                                                                   | `logs/issue-834-exp2-protocol-resize/protocol-resize-summary.json`                                                                                                 |
| Protocol mouse click                                     | `python3 scripts/test-issue-794-protocol-mouse.py --log-dir logs/issue-834-exp2-protocol-mouse-click --serve-bitcoin-pdf --action click`                                                                                                                                     | 0                                               | `first_failing_hop = "no-failure-observed"`                                                                   | `logs/issue-834-exp2-protocol-mouse-click/protocol-mouse-summary.json`                                                                                             |
| Protocol select/copy                                     | `python3 scripts/test-issue-794-protocol-mouse.py --log-dir logs/issue-834-exp2-protocol-select-copy --serve-bitcoin-pdf --action key-select-copy`                                                                                                                           | 0                                               | `first_failing_hop = "no-failure-observed"`                                                                   | `logs/issue-834-exp2-protocol-select-copy/protocol-mouse-summary.json`                                                                                             |
| PDF security guards                                      | `python3 scripts/test-issue-796-pdf-security.py --log-dir logs/issue-834-exp2-security`                                                                                                                                                                                      | 0                                               | `status = "pass"`                                                                                             | `logs/issue-834-exp2-security/issue-796-pdf-security-summary.json`                                                                                                 |
| Non-PDF smoke                                            | `python3 -m http.server 9791 --bind 127.0.0.1 --directory test-html/public`; `python3 scripts/test-issue-794-protocol-mouse.py http://127.0.0.1:9791/test-interactions.html --log-dir logs/issue-834-exp2-non-pdf-html --action click --url-contains test-interactions.html` | Server started successfully; probe 0; cleanup 0 | `first_failing_hop = "no-failure-observed"`                                                                   | `logs/issue-834-exp2-non-pdf-html/protocol-mouse-summary.json`; `logs/issue-834-exp2-non-pdf-html/http-server.log`                                                 |

Key evidence from the summaries:

- Save/download created `bitcoin.pdf`; title propagation was true; embedded PDF
  title behavior passed with the host title preserved; contained print produced
  four fresh intercept lines and did not submit a real print job.
- Local parity covered HTTP PDF, `file://` PDF, extensionless HTTP PDF,
  extensionless `file://` PDF, HTTP untitled PDF, and `file://` untitled PDF.
  The local parity harness records scroll/page-navigation there as diagnostics
  only; dedicated protocol scroll and toolbar-event probes remain the
  authoritative coverage for those behaviors.
- Toolbar event coverage passed for fit, zoom in, zoom out, and rotate. Each
  control was found, each click was observed, state changed, screenshots
  changed, and each row reported `firstFailingHop = "no-failure-observed"`.
- Protocol scroll sent six scroll events at plugin coordinates, captured before
  and after screenshots successfully, changed screenshot/state, and reported
  `first_failing_hop = "no-failure-observed"`.
- Protocol resize sent resize messages from `900x700` to `1300x900`; viewport
  dimensions changed by `200x100`; viewer/container geometry changed; before and
  after screenshots changed; and the summary reported
  `first_failing_hop = "no-failure-observed"`.
- Protocol mouse click sent down/up events at plugin coordinates; Roamium and
  PDFium input trace lines were present; state changed; and the summary reported
  `first_failing_hop = "no-failure-observed"`.
- Protocol select/copy sent mouse and key messages, produced non-empty PDFium
  selection evidence, and copied 21,230 bytes of Bitcoin PDF text to the
  clipboard from an initially empty clipboard.
- PDF security passed the positive-path labels (`process-per-site`,
  `process-map-insert`, `pdf-activate-request`, and `chrome-resources-grant`),
  confirmed forbidden fake-navigation labels were absent, and confirmed static
  sender guards for `resourcesPrivate` and `pdfViewerPrivate`.
- The non-PDF smoke loaded the local HTML fixture, clicked an HTML target rather
  than a PDF plugin target, changed screenshot/state, and reported
  `first_failing_hop = "no-failure-observed"`.

Current matrix deltas from this experiment:

| Feature                             | Roamium status after Experiment 2                         | Evidence                                                                                                                 |
| ----------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Full-page PDF rendering             | Proven                                                    | All PDF probes loaded `bitcoin.pdf` and captured viewer state without a failing setup hop.                               |
| Embedded PDF rendering              | Proven                                                    | `embeddedTitle.status = "pass"` and embedded plugin path evidence in the save/title/local summary.                       |
| HTTP PDFs                           | Proven                                                    | HTTP `bitcoin.pdf` used by all PDF probes.                                                                               |
| `file://` PDFs                      | Proven                                                    | Local parity entries include `file:///.../bitcoin.pdf` and `file:///.../untitled.pdf`.                                   |
| Extensionless PDFs                  | Proven                                                    | Local parity entries include HTTP and `file://` extensionless fixtures.                                                  |
| Scroll wheel                        | Proven                                                    | Dedicated protocol scroll probe passed with screenshot/state change and Roamium scroll trace evidence.                   |
| Mouse click and focus/input routing | Proven                                                    | Dedicated protocol click probe passed with Roamium/PDFium input trace evidence.                                          |
| Text selection and copy             | Proven                                                    | Dedicated select/copy probe copied 21,230 bytes from the PDF to the clipboard.                                           |
| Zoom in and zoom out                | Proven                                                    | Toolbar event probe passed for both controls.                                                                            |
| Fit modes                           | Proven                                                    | Toolbar event probe passed for fit.                                                                                      |
| Rotate                              | Proven                                                    | Toolbar event probe passed for rotate.                                                                                   |
| Save/download                       | Proven                                                    | Save/download probe created `bitcoin.pdf`.                                                                               |
| Title propagation                   | Proven                                                    | Save/title/local probe reported `titlePropagationPass = true`.                                                           |
| Native print containment            | Proven only for contained intercept path                  | Print probe reported `print-contained-callback`; real native print UI and real print jobs remain intentionally untested. |
| Security guards                     | Proven for current PDF activation/resource guard coverage | Issue 796 security probe passed positive, fake-navigation-negative, and static guard checks.                             |
| Non-PDF regression smoke            | Proven for click routing on the local interaction fixture | Non-PDF smoke passed against `test-interactions.html`.                                                                   |

Rows still not proven current by this experiment include keyboard page/scroll
navigation, toolbar page selector coverage, internal and external PDF links,
find/search, copy/save restrictions and disabled toolbar states,
password-protected PDFs, malformed/error PDFs, forms, annotations, context
menus, accessibility/searchify, real native print UI behavior, and split/tab/
window geometry with PDFs open.

No Chromium, Roamium, Ghostboard, Surfari, WebKit, protocol, fixture, or harness
source files were changed.

## Conclusion

The current Roamium core PDF baseline is still working. Experiment 2 upgrades
the already-known Roamium baseline from historical evidence to fresh
current-tree proof for rendering, embedded/local/extensionless load paths,
scroll, resize, click, selection/copy, toolbar fit/zoom/rotate, save/download,
title propagation, contained print safety, security guard coverage, and one
non-PDF regression smoke.

The next experiment should stay in the Roamium phase and target the first
remaining unproven workflow class, with keyboard page/scroll navigation and
toolbar page selector coverage as the most direct next candidates.

## Completion Review

Fresh-context adversarial review by Codex subagent `McClintock`: **Changes
required**, then **Approved** after fixes.

Required finding:

- The initial result table recorded each command as `Pass`, but did not
  explicitly record per-command exit statuses as required by the approved
  experiment plan.

Fix:

- Added an `Exit status` column for every command row, including explicit `0`
  statuses for each probe and the temporary HTTP server/probe/cleanup sequence
  for the non-PDF smoke.

Re-review verdict: **Approved**. The reviewer confirmed that the required
finding was resolved and that no new required findings were introduced.
