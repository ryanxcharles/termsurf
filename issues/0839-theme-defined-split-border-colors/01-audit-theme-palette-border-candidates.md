# Experiment 1: Audit theme palette border candidates

## Description

Audit whether the proposed palette-derived split border defaults are a
defensible first heuristic before changing runtime behavior.

The issue goal is to avoid modifying bundled theme files while still making
unset split border colors feel theme-native. The proposed first heuristic is:

- focused split border: palette index 6;
- unfocused split border: palette index 8.

This experiment will inspect the currently bundled/generated theme data and
produce an audit table that answers:

- whether every bundled theme has usable palette entries at indices 6 and 8;
- whether Tokyo Night derives the required colors from those entries;
- whether palette 6 and palette 8 are visible against each theme background;
- whether any obvious outliers need a different fallback before runtime code is
  changed.

No app runtime behavior changes in this experiment. If the audit supports the
heuristic, the next experiment will implement the Swift/Zig/doc changes and will
include targeted tests for nullability and override behavior.

## Changes

- `issues/0839-theme-defined-split-border-colors/README.md`
  - Link this audit experiment as the first experiment.
- `issues/0839-theme-defined-split-border-colors/01-audit-theme-palette-border-candidates.md`
  - Record the audit design.
  - After running the audit, append the audit command, summary table, result,
    conclusion, and design-review notes.

No source code, theme files, generated theme output, vendoring metadata, or
website docs should be changed in this experiment.

## Verification

1. Confirm the experiment is documentation-only:

   ```bash
   git diff --name-only | rg -v '^issues/0839-theme-defined-split-border-colors/|^issues/README.md$'
   ```

   Pass: no output.

2. Confirm no bundled/generated theme files or theme dependency metadata were
   changed:

   ```bash
   git status --short -- ghostboard/zig-out ghostboard/build.zig.zon
   git diff --name-only | rg '(^ghostboard/zig-out/|ghostboard/build.zig.zon|themes/)'
   ```

   Pass: no output.

3. Run an audit over the available bundled theme files, using generated theme
   output or the downloaded `iterm2_themes` package only as read-only data:

   ```bash
   find ghostboard/zig-out/share/ghostty/themes -type f -maxdepth 1 | wc -l
   ```

   Then parse each theme's `background`, `palette = 6=...`, and
   `palette = 8=...`, calculate WCAG-style contrast between each candidate and
   the background, and record:

   - theme count;
   - count missing palette 6;
   - count missing palette 8;
   - Tokyo Night palette 6 and 8 values;
   - lowest focused contrast sample;
   - lowest unfocused contrast sample;
   - number of candidate outliers needing manual review.

   Pass: all bundled themes expose palette 6 and 8; Tokyo Night exposes focused
   `#7dcfff` and unfocused `#414868`; any low-contrast outliers are identified
   explicitly so the next experiment can either accept the simple heuristic or
   add a fallback.

4. Format markdown and check whitespace:

   ```bash
   prettier --check issues/0839-theme-defined-split-border-colors/README.md \
     issues/0839-theme-defined-split-border-colors/01-audit-theme-palette-border-candidates.md \
     issues/README.md
   git diff --check
   ```

   Pass: all checks succeed.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes Required**.

- Required: the first experiment changed runtime behavior before performing the
  heuristic audit required by the issue.
- Required: the Zig/C nullability verification claim was too indirect because it
  did not require targeted tests for the two border color keys.

Fixes:

- Rewrote Experiment 1 as a documentation-only audit gate.
- Deferred Swift, Zig, and website documentation changes to the next experiment.
- Removed the indirect Zig/C nullability verification from this experiment.

Final verdict: **Approved**.
