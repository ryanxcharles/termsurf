# Experiment 20: Wire Roamium Browser Print Settings

## Description

Experiment 19 proved that Roamium native PDF print reaches
`PrintRenderFrameHelper::PrintNode()` and then stops at
`browser-default-print-settings-null`. The renderer print helper asks the
browser process for default print settings, receives null, and aborts before a
native dialog can appear.

This experiment should wire the missing browser-side print settings/manager path
for Roamium WebContents, then rerun the guarded native print probe. The target
is the smallest Chromium integration needed for `GetDefaultPrintSettings()` to
return usable settings and for the print path to advance past
`print-init-settings-failed`.

## Changes

1. Create a fresh Chromium branch for this issue experiment.

   Follow `chromium/AGENTS.md`:

   ```bash
   cd chromium/src
   git checkout 148.0.7778.97-issue-834-exp8
   git checkout -b 148.0.7778.97-issue-834-exp20
   ```

   If a newer Issue 834 Chromium branch already exists by the time this
   experiment runs, branch from the most relevant recent Issue 834 branch
   instead. Update the branch table in `chromium/README.md`.

2. Wire browser-side print manager ownership for Roamium WebContents.

   The static audit found:

   - `PrintViewManagerBasic::BindPrintManagerHost()` binds renderer
     `PrintManagerHost` requests only if
     `PrintViewManagerBasic::FromWebContents` returns a manager;
   - `PrintViewManager::BindPrintManagerHost()` does the same for print-preview
     builds;
   - TermSurf currently creates renderer-side `PrintRenderFrameHelper` but does
     not create browser-side `PrintViewManager` / `PrintViewManagerBasic`
     ownership for Roamium WebContents.

   Add the narrowest TermSurf integration that:

   - creates the correct browser-side print manager for each Roamium
     WebContents;
   - registers/binds `printing::mojom::PrintManagerHost` for render frames;
   - creates any required print-composite client or supporting print WebContents
     user data expected by Chromium's print manager;
   - uses upstream Chromium classes instead of custom fake settings.

   Prefer print preview if Roamium's Chromium build has print preview enabled.
   Otherwise use Chromium's basic print manager path. Do not invent a TermSurf
   print settings stub unless the upstream manager path cannot work and the
   result explains why.

3. Preserve existing safety gates.

   The native print probe must still:

   - run the harmless preflight before production print;
   - require `--allow-native-dialog-click`;
   - capture print queue before and after;
   - cancel any observed native print dialog;
   - fail if a print job is submitted unexpectedly.

4. Run the guarded native print probe.

   ```bash
   python3 scripts/test-issue-834-pdf-native-print.py \
     --log-dir logs/issue-834-exp20-browser-print-settings \
     --probe native-dialog \
     --allow-native-dialog-click
   ```

   Advancing past `browser-default-print-settings-null` is only Partial unless
   Roamium opens a native dialog, the safety watcher cancels it, and print queue
   evidence proves no print job was submitted. If the path advances but stops at
   a later print manager/dialog hop, record Partial with that new first failing
   hop.

## Verification

Verification for the completed result is:

```bash
git status --short
git -C chromium/src status --short
git -C chromium/src rev-parse --abbrev-ref HEAD
git -C chromium/src rev-parse HEAD
git diff --check

cd chromium/src
export PATH="/Users/astrohacker/dev/termsurf/chromium/depot_tools:$PATH"
autoninja -C out/Default libtermsurf_chromium

cd /Users/astrohacker/dev/termsurf
rm -rf scripts/__pycache__
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-pdf-native-print.py
rm -rf scripts/__pycache__
node --check scripts/probe-pdf-save-print-title-local.mjs

python3 scripts/test-issue-834-pdf-native-print.py \
  --log-dir logs/issue-834-exp20-browser-print-settings \
  --probe native-dialog \
  --allow-native-dialog-click

git diff --check
```

After committing the Chromium branch changes inside `chromium/src`, regenerate
the cumulative Issue 834 patch archive:

```bash
cd /Users/astrohacker/dev/termsurf/chromium/src
rm -rf ../../chromium/patches/issue-834
git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-834
```

Required evidence:

- `chromium/README.md` records the new Chromium branch;
- `chromium/src` is on the expected Issue 834 branch;
- Chromium source changes are committed inside `chromium/src`;
- `autoninja -C out/Default libtermsurf_chromium` passes;
- the Issue 834 patch archive is regenerated;
- the guarded native print probe records its internal preflight;
- no print job is submitted;
- the result records whether the first failing hop advanced past
  `browser-default-print-settings-null`;
- if a native dialog appears, it is cancelled and queue state remains unchanged;
- if no native dialog appears, the new first failing hop is classified from
  source and log evidence;
- markdown is formatted with Prettier;
- Python bytecode cache is removed after compilation;
- `git diff --check` passes;
- design review is recorded, all real design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if Roamium native PDF print opens a native print dialog,
the safety watcher cancels it, and print queue evidence proves no job was
submitted.

## Partial Criteria

This experiment is partial if browser-side print settings are no longer null but
the path stops at a later print-preview, print-manager, macOS dialog, or watcher
hop. Partial is also acceptable if upstream Chromium's print manager path cannot
be integrated in one experiment and the result identifies the next missing hop
with source and log evidence.

## Failure Criteria

This experiment fails if it submits a print job, weakens the native print safety
gate, bypasses Chromium's normal print manager with an unjustified fake settings
stub, leaves Chromium branch/patch records inconsistent, or claims native print
support without native dialog and no-job evidence.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes Required**.

Required finding:

- The probe step said the experiment passed if the first failing hop advanced
  past `browser-default-print-settings-null`, which conflicted with the stricter
  pass criteria requiring native dialog cancellation and no-job evidence.

Fix:

- Reworded the probe step to state that advancing past
  `browser-default-print-settings-null` is only Partial unless the native dialog
  opens, the safety watcher cancels it, and queue evidence proves no print job
  was submitted.

Re-review verdict: **Approved**.

The reviewer found no remaining Required findings.

## Result

**Result:** Partial

Implemented a narrow Roamium PDF print manager instead of linking Chromium's
full Chrome browser print UI target. The broad `//chrome/browser/printing:impl`
approach was tried first and rejected because it pulled in unrelated Chrome UI,
enterprise, media, and duplicate PDF stream-manager dependencies, causing
`libtermsurf_chromium.dylib` link failures.

The retained implementation adds `termsurf::TsPdfPrintManager`, creates it for
each Roamium `WebContents`, binds `printing::mojom::PrintManagerHost` for PDF
renderer frames, and uses Chromium's exported `PrintingContext` /
`RenderParamsFromPrintSettings()` path to produce non-null default print
settings. The manager preserves the initialized print request by document cookie
between `GetDefaultPrintSettings()` and `ScriptedPrint()`, matching the upstream
lifecycle where Chromium queues the initialized `PrinterQuery` after default
settings and consumes it during the final user-settings step.

The guarded native print probe now advances past the Experiment 19 blocker:

```text
get-default-print-settings-enter
get-default-print-settings-ok
print-init-settings-ok
did-show-print-dialog-enter
did-show-print-dialog-exit
scripted-print-enter
scripted-print-settings-null
```

The old first failing hop, `browser-default-print-settings-null`, is gone. The
new first failing hop is `native-print-click-sent-no-dialog`: the toolbar print
click is sent, default print settings are valid, the renderer enters
`ScriptedPrint()`, but no native macOS print dialog is observed and the scripted
print settings response is null.

The safety gate remained intact. The preflight dialog watcher passed before the
production print click. The print queue was empty before and after the probe,
and no print job was submitted.

Verification evidence:

- `autoninja -C out/Default libtermsurf_chromium` passed on
  `148.0.7778.97-issue-834-exp20` after the preserved-request fix.
- Chromium source commit: `453d826490b5134341fce36bb199c34843bd421e`.
- The cumulative Issue 834 patch archive was regenerated from the visible
  Chromium 148.0.7778.97 shallow base commit
  `6b3fa66a923a9442c8ab0bc71b4b41ff24528d3b`, because this checkout does not
  have a local `148.0.7778.97` ref.
- `scripts/test-issue-834-pdf-native-print.py --log-dir logs/issue-834-exp20-browser-print-settings --probe native-dialog --allow-native-dialog-click`
  completed with `probe_status = "ok"`, `safety_gate_passed = true`, and
  `first_failing_hop = "native-print-click-sent-no-dialog"`.
- `logs/issue-834-exp20-browser-print-settings/pdf-native-print.log` records
  `get-default-print-settings-ok` and `scripted-print-settings-null`.

## Conclusion

Experiment 20 proves Roamium was missing browser-side `PrintManagerHost`
ownership/settings support and fixes that layer. Native print is still not
complete: the next experiment should focus specifically on why
`PrintingContext::AskUserForSettings()` does not produce an observable macOS
print dialog in the Roamium embedding path, despite the renderer reaching
`scripted-print-enter`.

## Completion Review

An adversarial Codex subagent reviewed the completed result with fresh context.

Initial verdict: **Changes Required**.

Required finding:

- `TsPdfPrintManager` returned default settings but destroyed the initialized
  print request before `ScriptedPrint()`. Upstream Chromium preserves the
  initialized `PrinterQuery` after default settings and pops it by cookie during
  the final user-settings step.

Fix:

- Added `pending_print_requests_`, keyed by document cookie, so
  `GetDefaultPrintSettings()` preserves the initialized request and
  `ScriptedPrint(params->cookie)` consumes the matching request instead of
  creating a fresh context.

Re-review verdict: **Approved**.

The reviewer confirmed the prior Required finding is resolved and found no new
Required findings.
