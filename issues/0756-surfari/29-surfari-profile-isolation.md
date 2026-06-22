# Experiment 29: Prove Surfari profile isolation

## Description

Experiment 28 completed the input-detail tranche. The next missing real-app
matrix row is `Profile isolation`.

This experiment should prove that Surfari profiles are isolated at the same
boundaries Roamium profiles are isolated:

- Ghostboard routes each `web --browser surfari --profile <name>` request to a
  profile/browser-specific server key.
- Ghostboard spawns separate Surfari processes for different profiles with
  distinct `--user-data-dir` paths under `webkit-profiles`.
- Surfari registers back with the matching `ServerRegister.profile` and
  `ServerRegister.browser`.
- Browser storage state for one profile does not leak into another profile,
  including both `localStorage` and cookies.
- Returning to the first profile restores its own same-origin browser storage.
- Hit testing and keyboard input route only to the selected Surfari profile's
  pane/context.

Use the existing Roamium `multi-profile-isolation` scenario in
`scripts/ghostboard-geometry-matrix.sh` as the reference, but keep this
experiment Surfari-specific and focused. Do not expand into crash handling or
the final Ghostboard/Roamium comparison.

Current inspection found an expected failure point to verify:
`surfari/src/main.rs` parses `--user-data-dir` only to infer the profile name,
then calls `ts_create_browser_context(ptr::null())`; `libtermsurf_webkit`
currently ignores the `path` argument and uses
`[WKWebsiteDataStore defaultDataStore]` for every normal context. The harness
should first expose whether named Surfari profiles actually share WebKit storage
today. If they do, fix that narrow storage boundary instead of changing
Ghostboard, WebTUI, or the TermSurf protocol.

## Changes

- Add a focused Surfari profile-isolation harness under `scripts/`.
- Launch the real Debug `TermSurf.app` with repo-built `web --browser surfari`
  and repo-built `surfari`, explicitly setting
  `TERMSURF_SURFARI_PATH=$ROOT/target/debug/surfari` for Ghostboard.
- Serve a deterministic same-origin fixture that writes and logs a profile
  marker in both `localStorage` and `document.cookie`. The page may use a query
  parameter to decide which marker to write, but profile A and profile B must
  share the same HTTP origin so the test proves storage isolation rather than
  origin isolation.
- Run the profile sequence adapted from Roamium:
  - launch profile A with `--profile profilea`;
  - assert Ghostboard creates `profilea/surfari`;
  - assert the spawn line uses path `$ROOT/target/debug/surfari` and
    `--user-data-dir=.../webkit-profiles/profilea`;
  - assert Surfari registers as `profile=profilea browser=surfari`;
  - assert the fixture logs `localStorage before=none after=profilea` and
    `cookie before=none after=profilea`;
  - open a new native terminal tab;
  - launch profile B with `--profile profileb`;
  - assert Ghostboard creates `profileb/surfari`;
  - assert the spawn line uses path `$ROOT/target/debug/surfari` and
    `--user-data-dir=.../webkit-profiles/profileb`;
  - assert profile B uses a different Surfari process and CA context;
  - assert the fixture logs `localStorage before=none after=profileb` and
    `cookie before=none after=profileb`;
  - prove profile B hit testing and keyboard input do not route to profile A;
  - switch back to profile A, reload, and assert the fixture logs
    `localStorage before=profilea after=profilea` and
    `cookie before=profilea after=profilea`;
  - prove profile A hit testing and keyboard input do not route to profile B.
- If storage isolation fails while server routing succeeds, fix Surfari's
  browser-context creation path so `--user-data-dir` reaches
  `libtermsurf_webkit` and named profiles use persistent, profile-specific
  WebKit storage.
- Keep any fix narrowly scoped to Surfari profile storage unless evidence shows
  the failure is in a different boundary.
- Update `issues/0756-surfari/real-app-matrix.md` only if the experiment
  directly proves the `Profile isolation` row.

## Verification

Pass criteria:

- Build or confirm required artifacts:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
```

- Run the new Surfari profile-isolation harness.
- The harness must prove, in the real app:
  - profile A and profile B create distinct Ghostboard server keys;
  - profile A and profile B spawn distinct Surfari processes;
  - both spawn lines use `$ROOT/target/debug/surfari` and profile-specific
    `webkit-profiles/<profile>` paths;
  - both Surfari processes register with matching profile/browser names;
  - profile A and profile B use distinct pane/context IDs;
  - profile B's first same-origin `localStorage` and cookie reads are `none`,
    not profile A's marker;
  - returning to profile A and reloading reads profile A's `localStorage` marker
    and cookie marker;
  - keyboard and hit testing route only to the selected profile's Surfari
    context.
- The harness must fail if profile B observes profile A's `localStorage` marker
  or cookie marker.
- The harness must fail if returning to profile A loses profile A's
  `localStorage` marker or cookie marker.
- The harness must fail if Ghostboard spawns any Surfari process from a path
  other than `$ROOT/target/debug/surfari`.
- The harness must fail if a profile routes input to the other profile's pane or
  context.
- Run hygiene checks:

```bash
git diff --check
bash -n <new-surfari-profile-isolation-harness>
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/29-surfari-profile-isolation.md \
  issues/0756-surfari/real-app-matrix.md
```

Run formatting/checks for any source files touched:

```bash
cargo fmt -- <rust-files>
zig fmt <zig-files>
```

Result classification:

- `Pass` means profile server routing, process separation, profile-specific
  user-data directories, same-origin storage isolation/persistence, and
  profile-specific input routing are all directly proven in the real app,
  allowing `Profile isolation` to become `Proven`.
- `Partial` means Ghostboard/Surfari profile routing is proven but browser
  storage isolation or input routing remains unproven or broken.
- `Fail` means the harness cannot launch the Surfari profile sequence or cannot
  produce stronger profile evidence than the existing matrix.

## Design Review

Adversarial design review initially returned `CHANGES REQUIRED` with two
Required findings:

- The plan overclaimed profile isolation because the matrix requires
  localStorage, cookies, and state, but the first draft only required a generic
  storage marker.
- The plan did not explicitly require Ghostboard to launch the repo-built
  Surfari binary through `TERMSURF_SURFARI_PATH`, nor assert the spawned path.

The design was updated to require both `localStorage` and cookie
isolation/persistence, to launch Ghostboard with
`TERMSURF_SURFARI_PATH=$ROOT/target/debug/surfari`, and to fail if any Surfari
spawn uses another binary path.

Focused re-review returned `APPROVED` with no Required findings. The reviewer
confirmed both prior findings were resolved and no new Required finding was
introduced.

## Result

**Result:** Pass

Run `20260621-203102` passed with the new real-app profile-isolation harness:

```bash
bash -n scripts/test-issue-756-surfari-profile-isolation.sh
scripts/test-issue-756-surfari-profile-isolation.sh
```

Logs:

- `logs/issue-756-exp29-surfari-profile-isolation/harness-20260621-203102.log`
- `logs/issue-756-exp29-surfari-profile-isolation/app-20260621-203102.log`
- `logs/issue-756-exp29-surfari-profile-isolation/surfari-trace-20260621-203102.log`
- `logs/issue-756-exp29-surfari-profile-isolation/webtui-20260621-203102.log`

The harness launched the real Debug `TermSurf.app` with repo-built
`web --browser surfari` and repo-built `surfari`, explicitly pinning Ghostboard
with `TERMSURF_SURFARI_PATH=$ROOT/target/debug/surfari`. It also set temporary
`XDG_DATA_HOME` and `XDG_STATE_HOME` paths so stale user profile data could not
make the storage assertions pass accidentally.

The passing run proved:

- profile A created Ghostboard server key `profilea/surfari`;
- profile A spawned `/Users/astrohacker/dev/termsurf/target/debug/surfari` with
  `--user-data-dir=.../webkit-profiles/profilea`;
- profile A registered as `profile=profilea browser=surfari`;
- profile A initially logged same-origin
  `localStorage_before=none localStorage_after=profilea` and
  `cookie_before=none cookie_after=profilea`;
- profile B created Ghostboard server key `profileb/surfari`;
- profile B spawned the same repo Surfari binary with
  `--user-data-dir=.../webkit-profiles/profileb`;
- profile B used a different Surfari PID and CA context from profile A;
- profile B registered as `profile=profileb browser=surfari`;
- profile B initially logged same-origin
  `localStorage_before=none localStorage_after=profileb` and
  `cookie_before=none cookie_after=profileb`;
- profile B hit testing and keyboard input routed to profile B and not profile
  A;
- after switching back to profile A and explicitly navigating profile A to the
  same URL, the page logged
  `localStorage_before=profilea localStorage_after=profilea` and
  `cookie_before=profilea cookie_after=profilea`;
- profile A return hit testing and keyboard input routed to profile A and not
  profile B;
- both temporary `webkit-profiles/profilea` and `webkit-profiles/profileb`
  directories existed under the run's `XDG_DATA_HOME`.

Implementation changes:

- Added `scripts/test-issue-756-surfari-profile-isolation.sh`.
- Updated `surfari/src/main.rs` to retain `--user-data-dir` and pass it to
  `ts_create_browser_context`.
- Updated `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm` so normal
  browser contexts create a WebKit data store with profile-specific storage,
  cache, localStorage, IndexedDB, cache storage, service worker, and cookie
  paths instead of always using `[WKWebsiteDataStore defaultDataStore]`.

The first harness run, `20260621-202814`, failed on the return-to-profile-A
proof because Browse-mode `cmd+r` was forwarded as a key event but did not
produce a reload/navigation. That failure did not show storage loss. The harness
was changed to use the direct `BrowserNavigate` protobuf path already used by
Experiment 25's lifecycle tranche, which re-runs the same profile A page script
without depending on the separate reload shortcut.

## Conclusion

Surfari now has real-app evidence for named profile isolation. The real-app
matrix marks `Profile isolation` `Proven`. Remaining Issue 756 matrix gaps are
crash handling and the final Ghostboard/Roamium comparison.

## Completion Review

Adversarial completion review returned `APPROVED` with no findings. The reviewer
inspected the uncommitted result diff, the new profile-isolation harness, the
passing `20260621-203102` run logs, the issue README, and the real-app matrix.
The reviewer confirmed that the result stayed within the approved Experiment 29
scope, proved localStorage and cookie isolation with separate Surfari processes
and profile-specific user-data directories, pinned Ghostboard to the repo-built
Surfari binary, updated the matrix without overclaiming, and had not been
committed before review.
