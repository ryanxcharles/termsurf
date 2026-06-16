# Experiment 196: Live bell audio playback

## Description

Experiment 195 narrowed user-notification delivery to the current VM's denied
notification authorization boundary. The remaining `RUNTIME-012B2B2B2B2B3C` gap
still includes audible bell output and OS-visible Dock attention state.

This experiment targets only the audio bell slice. Existing guards prove the
bell config bridge, the `bell-features = audio` branch, the configured
`bell-audio-path`, and the app trace around `NSSound(contentsOfFile:)` plus
`sound.play()`. They do not prove that AppKit accepted playback or that the
sound completed.

The goal is to prove deterministic `NSSound` playback acceptance and completion
for the bell audio path. If the VM cannot expose actual physical audio output,
the result must keep physical audibility out of the claim and record the exact
remaining audio boundary.

## Changes

- Add a focused live guard, tentatively
  `issues/0805-roastty-ghostty-parity/macos_live_bell_audio_playback.py`.
  - Launch the built debug Roastty app with isolated config/defaults,
    `bell-features = no-system,audio,no-attention,no-title,no-border`, and a
    deterministic short audio file as `bell-audio-path`.
  - Emit BEL from a controlled terminal command.
  - Add env-gated trace evidence around the production `NSSound` branch only if
    needed: sound object creation, `play()` return value, `isPlaying` shortly
    after playback starts, and delegate/completion callback or timeout.
  - Use an otherwise identical `no-audio` control run to prove the audio trace
    is gated by `bell-features = audio`.
  - Check for new Roastty crash reports.
- Update `config_runtime_inventory.py` according to the result:
  - If AppKit playback acceptance/completion passes, split a new Oracle-complete
    row from `RUNTIME-012B2B2B2B2B3C` for live `NSSound` audio playback
    acceptance/completion.
  - Keep physical audible speaker output separate unless the guard has a real
    audio-capture oracle.
  - If playback acceptance or completion cannot be observed, leave an exact gap
    naming the failing OS/audio boundary.
- Update `notification_link_bell_gui_residual_parity.py` to enforce the new row
  split or exact audio-boundary wording.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update Issue 805 `README.md` Learnings and Experiments index after the result
  is known.

## Verification

Pass criteria:

- The guard proves exact debug-app launch, isolated config/defaults, controlled
  BEL emission, and no new Roastty crash report.
- The audio-enabled run proves `ringBell target=surface`,
  `appBell system=false audio=true attention=false`, the expected
  `bell-audio-path`, `sound.play()` acceptance, and an observed `NSSound`
  completion callback.
- The audio-disabled control proves the same BEL path with
  `appBell system=false audio=false attention=false` and no audio playback
  evidence.
- The result distinguishes AppKit/NSSound playback acceptance/completion from
  physical audible speaker output.
- A timeout or missing completion callback is not a pass condition. It may only
  be recorded as `Partial` or `Fail` evidence naming the unresolved OS/audio
  boundary.
- Inventory counts and remaining gap IDs are updated exactly and asserted by
  guards.

Automation constraint discovered before implementation:

- On this macOS VM, launching the Exp196 audio guard can trigger a TCC prompt:
  `"Roastty" would like to access the Microphone.` The prompt text is declared
  by `INFOPLIST_KEY_NSMicrophoneUsageDescription` in
  `roastty/macos/Roastty.xcodeproj/project.pbxproj`.
- The prompt is not exposed as a normal System Events `Allow` button and must
  not be a required human step for an unattended regression guard.
- Before claiming a pass, revise the implementation so the durable guard avoids
  depending on accepting this microphone prompt, or record the audio-device path
  as an OS/TCC-gated residual with exact evidence.
- The guard runner can now capture such prompts after granting screenshot
  permission to the host terminal app, but prompt visibility is diagnostic
  evidence only, not a viable automation dependency.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_bell_audio_playback.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/196-live-bell-audio-playback.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The result must state the exact runtime row count, Oracle-complete count, closed
count, incomplete count, gap count, CFG-223 status, and remaining gap IDs.

## Design Review

Fresh-context Codex adversarial reviewer `Aristotle the 3rd` reviewed the
initial design and returned **Changes Required**.

Required finding accepted: the initial pass criteria allowed a timeout to count
alongside an `NSSound` completion callback, which would not prove the stated
goal of playback completion. The design now requires `sound.play()` acceptance
and an observed completion callback for an Oracle-complete split; timeout can
only be recorded as `Partial` or `Fail` boundary evidence.

## Result

**Result:** Partial

The first implementation attempt added an env-gated trace around the production
`NSSound(contentsOfFile:)` branch and a live guard that launched Roastty with
`bell-features = no-system,audio,no-attention,no-title,no-border`, generated a
short WAV file, and emitted BEL from a controlled child process. That guard did
not reach deterministic `NSSound.play()` acceptance/completion proof on this VM
because the debug app triggered a macOS TCC prompt before unattended execution
could proceed:

```text
"Roastty" would like to access the Microphone.
```

The prompt is not incidental test UI. Roastty's Xcode project declares
`INFOPLIST_KEY_NSMicrophoneUsageDescription = "A program running within Roastty would like to use your microphone.";`,
which matches the prompt text. The debug bundle identifier is
`com.mitchellh.roastty.debug`; resetting microphone permission for that
identifier and rebuilding reproduced the prompt. After screen-recording
permission was granted to the host terminal app, the guard runner could capture
the prompt at `/tmp/termsurf-roastty-microphone-prompt.png`, but System Events
did not expose the dialog's `Allow` button as a normal scriptable UI element.

The unfinished `NSSound` delegate/test hook was removed instead of committing a
guard that depends on human TCC approval. The only source change retained by
this result updates `bell_presentation_runtime_parity.py` so its stale
post-Experiment-192 assertions match the current generated inventory: CFG-223
has 94 Oracle-complete runtime rows, 97 closed rows, 1 incomplete row, 1 gap
row, and the remaining bell residual names OS-visible Dock attention state
beyond AppKit request dispatch rather than already-split link-preview or Launch
Services behavior.

Current CFG-223 counts remain unchanged:

- runtime rows: 98
- Oracle-complete runtime rows: 94
- closed rows: 97
- incomplete rows: 1
- gap rows: 1
- CFG-223 status: `Gap`
- remaining gap ID: `RUNTIME-012B2B2B2B2B3C`

Commands run:

```bash
tccutil reset Microphone com.mitchellh.roastty.debug
(cd roastty && macos/build.nu --action build) > logs/issue805-exp196-build-permission-reset.log 2>&1
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_bell_audio_playback.py > logs/issue805-exp196-bell-audio-permission-visible.log 2>&1 &
screencapture -x /tmp/termsurf-roastty-microphone-prompt.png
osascript -e 'tell application "System Events" to return UI elements enabled'
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py
python3 -m py_compile issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py issues/0805-roastty-ghostty-parity/config_runtime_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/196-live-bell-audio-playback.md
git diff --check
```

The build completed successfully. The live audio guard was stopped after the TCC
prompt appeared because proceeding would require a human to grant microphone
access to the rebuilt debug app identity. The durable evidence for the TCC
boundary is the successful build log plus the captured prompt screenshot; the
redirected live guard log is empty because the run was stopped while the prompt
was visible.

## Completion Review

Fresh-context Codex adversarial reviewer `Mendel the 3rd` reviewed the completed
experiment result and returned **Approved** with no required findings.

Optional finding accepted: the reviewer noted that
`logs/issue805-exp196-bell-audio-permission-visible.log` is empty and therefore
does not independently support the TCC-stop narrative. The result now clarifies
that the durable evidence is the successful build log and
`/tmp/termsurf-roastty-microphone-prompt.png`, not the redirected live guard
log.

The reviewer independently checked that the README status matches the `Partial`
result, `bell_presentation_runtime_parity.py` passes, markdown formatting
passes, `git diff --check` passes, the diff is limited to the experiment file,
README status, and static guard update, and no unfinished AppDelegate hooks or
`macos_live_bell_audio_playback.py` helper remain in the working diff.

## Conclusion

Experiment 196 did not split a new Oracle-complete audio playback row. It
established that the direct `NSSound` live-audio proof path is TCC-gated in this
macOS VM and is not suitable as an unattended regression guard. The remaining
`RUNTIME-012B2B2B2B2B3C` gap should either be resolved by a different proof that
does not initialize the OS microphone permission surface, or by explicitly
classifying the physical OS audio-device effect as an OS/TCC-controlled boundary
with enough source-parity and dispatch evidence to close it without a human
permission click.
