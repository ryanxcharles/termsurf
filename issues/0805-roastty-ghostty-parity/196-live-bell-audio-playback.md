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
