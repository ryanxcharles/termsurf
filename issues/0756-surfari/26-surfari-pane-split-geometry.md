# Experiment 26: Run Surfari pane and split geometry

## Description

Experiment 25 proved the single-window, single-pane lifecycle tranche. The next
highest-risk gap is pane geometry: Surfari must stay attached to the correct
terminal pane when Ghostboard creates splits and resizes panes.

This experiment should execute the pane/split portion of the real-app matrix
without expanding into tab switching, multi-window routing, cross-pane focus
state, profile isolation, crash handling, click/drag parity, or the final
Roamium comparison. Those remain separate rows and should get their own
experiments.

The implementation should reuse the existing Roamium geometry matrix as the
behavioral source of truth, especially the `split-right`, `split-down`, and
`split-right-resize` scenarios in `scripts/ghostboard-geometry-matrix.sh`, but
it should add a focused Surfari-specific harness instead of mutating the large
Roamium runner.

## Changes

- Add a focused Surfari pane/split geometry harness under `scripts/`.
- Launch the real Debug `TermSurf.app` with repo-built `web --browser surfari`
  and repo-built `surfari`, using deterministic local fixtures.
- Run small, independent real-app scenarios so one geometry state does not
  contaminate the next:
  - split right;
  - split down;
  - split right followed by divider resize.
- For each scenario, prove:
  - initial `BrowserReady` and AppKit overlay presentation;
  - the split action is dispatched by real Ghostboard keybinding or protocol
    path;
  - the original Surfari overlay moves/resizes to the new pane frame;
  - AppKit `presented` and `presented_pixels` logs agree on the new pane;
  - Surfari receives `resize ... ffi=ts_set_view_size` for the browser tab and
    pane with the new pixel size;
  - hit testing inside the new overlay frame maps to the Surfari context;
  - hit testing in the sibling pane area does not map to the Surfari context.
- For divider resize, additionally prove the resized split produces a second
  changed overlay frame/pixel size and a second Surfari resize.
- Update `issues/0756-surfari/real-app-matrix.md` only for directly proven rows:
  - mark `Pane resize` `Proven` only if divider resize passes;
  - mark `Split panes` `Proven` only if both right and down split behavior
    passes, including sibling negative hit tests.

## Verification

Pass criteria:

- Required builds/artifacts exist:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
```

- Run the new pane/split geometry harness.
- The harness must prove, in the real app, right split, down split, and
  right-split divider resize.
- The harness must fail if it only observes old baseline geometry, if Surfari
  does not receive the new pixel resize, or if sibling-pane hit tests still hit
  the browser overlay.
- Update `real-app-matrix.md` only for rows directly proven by this experiment.
- Run hygiene checks:

```bash
git diff --check
bash -n <new-pane-split-geometry-harness>
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/26-surfari-pane-split-geometry.md \
  issues/0756-surfari/real-app-matrix.md
```

Result classification:

- `Pass` means both split directions and divider resize are directly proven in
  the real app, allowing `Split panes` and `Pane resize` to become `Proven`.
- `Partial` means at least one pane/split behavior is proven but one or more of
  split right, split down, divider resize, Surfari resize, or sibling negative
  hit testing remains unproven.
- `Fail` means the harness cannot launch Surfari or cannot produce stronger
  pane/split geometry evidence than the existing matrix.

## Design Review

Adversarial design review returned `APPROVED` with no Required findings. The
reviewer confirmed that the README links Experiment 26 as `Designed`, the
experiment has Description, Changes, and Verification sections, the scope is
limited to split right, split down, and right-split divider resize, the design
explicitly excludes tabs, windows, focus, profile isolation, crash handling,
click/drag parity, and final Roamium comparison, the verification criteria cover
frame/pixel geometry, Surfari resize, and positive/negative hit testing, the
plan uses Roamium geometry scenarios as the source of truth without mutating the
large Roamium runner, hygiene/build checks are present, and the plan commit had
not already been made.

## Result

**Result:** Pass

Added `scripts/test-issue-756-surfari-pane-split-geometry.sh` and ran it
successfully with run ID `20260621-191750`.

Evidence:

- `logs/issue-756-exp26-surfari-pane-split/harness-20260621-191750.log`
- `logs/issue-756-exp26-surfari-pane-split/app-split-right-20260621-191750.log`
- `logs/issue-756-exp26-surfari-pane-split/surfari-split-right-20260621-191750.log`
- `logs/issue-756-exp26-surfari-pane-split/app-split-down-20260621-191750.log`
- `logs/issue-756-exp26-surfari-pane-split/surfari-split-down-20260621-191750.log`
- `logs/issue-756-exp26-surfari-pane-split/app-split-right-resize-20260621-191750.log`
- `logs/issue-756-exp26-surfari-pane-split/surfari-split-right-resize-20260621-191750.log`

The harness proved the pane/split tranche inside the real Debug `TermSurf.app`:

- `split-right` launched Surfari, dispatched the real `ctrl+d=new_split:right`
  keybinding, observed the Surfari overlay shrink from `880x510` to `424x510`,
  observed AppKit pixels change from `1760x1020` to `848x1020`, verified Zig
  recorded those AppKit pixels, verified Surfari received
  `resize ... pixel_width=848 pixel_height=1020 ... ffi=ts_set_view_size`,
  verified an inside click hit the Surfari context, and verified a sibling-pane
  click did not route to the Surfari context.
- `split-down` launched Surfari, dispatched the real `ctrl+j=new_split:down`
  keybinding, observed the Surfari overlay shrink from `880x510` to `880x187`,
  observed AppKit pixels change from `1760x1020` to `1760x374`, verified Zig
  recorded those AppKit pixels, verified Surfari received
  `resize ... pixel_width=1760 pixel_height=374 ... ffi=ts_set_view_size`,
  verified an inside click hit the Surfari context, and verified a sibling-pane
  click did not route to the Surfari context.
- `split-right-resize` launched Surfari, dispatched the real right split, then
  dispatched `ctrl+l=resize_split:right,20`, observed the Surfari overlay grow
  from `424x510` to `448x510`, observed AppKit pixels change to `896x1020`,
  verified Zig recorded those AppKit pixels, verified Surfari received
  `resize ... pixel_width=896 pixel_height=1020 ... ffi=ts_set_view_size`,
  verified an inside click hit the Surfari context, and verified a sibling-pane
  click did not route to the Surfari context.

Updated `issues/0756-surfari/real-app-matrix.md` conservatively:

- `Split panes` is now `Proven` for single-browser right/down split geometry.
- `Pane resize` is now `Proven` for right-split divider resize.

## Conclusion

Surfari now has direct real-app evidence for pane/split geometry: right splits,
down splits, and right-split divider resize move and resize the overlay,
propagate the new pixel size to Surfari, keep positive hit testing attached to
the browser pane, and reject sibling-pane hit testing.

The next experiment should cover tab/window/focus geometry. It should not claim
profile isolation, crash handling, click/drag parity, or final Roamium
comparison until those rows have their own direct evidence.

## Completion Review

Adversarial completion review returned `APPROVED` with no findings. The reviewer
confirmed that the result commit had not been made, `git diff --check`,
`bash -n scripts/test-issue-756-surfari-pane-split-geometry.sh`, and Prettier
checks passed, the saved run logs support the `Pane resize` and `Split panes`
matrix updates, and the result does not smuggle claims for tabs, windows, focus,
profiles, crashes, click/drag parity, or final Roamium comparison.
