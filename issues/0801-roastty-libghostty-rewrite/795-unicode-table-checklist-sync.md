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

# Experiment 795: Unicode Table Checklist Sync

## Description

Issue 801's supporting subsystem and dependency checklists still say the
Unicode/uucode table work has no tables. That is stale for the current font
path: `roastty/src/font/emoji_presentation.rs` is a generated
`Emoji_Presentation` table from `vendor/uucode/ucd/emoji/emoji-data.txt`, and
`roastty/src/font/codepoint_resolver.rs` uses it to choose the default
emoji/text presentation.

This experiment updates the checklist wording only. It does not mark the rows
complete because the broader Ghostty `uucode` surface still includes Unicode
property, grapheme-break, and width/wcwidth tables that do not exist as a
complete first-class `unicode/` namespace yet.

## Changes

- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Scope the top-level `uucode` dependency-policy bullet so it describes how
    Unicode tables should be generated as they are ported, rather than implying
    all Unicode tables already exist.
  - Update the `unicode/` supporting subsystem row from "missing (no tables)" to
    scoped partial wording that names the existing `Emoji_Presentation` table
    and leaves grapheme-break, width/wcwidth, broader property tables, and a
    dedicated namespace open.
  - Update the `uucode` dependency row from "not started (no Unicode tables
    exist yet)" to scoped partial wording that names the generated
    `Emoji_Presentation` table and leaves the rest of the table set open.
  - Add the Experiment 795 index entry.
- `issues/0801-roastty-libghostty-rewrite/795-unicode-table-checklist-sync.md`
  - Record verification evidence and review results.

## Verification

- Inspect:
  - `roastty/src/font/emoji_presentation.rs`
  - `roastty/src/font/codepoint_resolver.rs`
  - `roastty/src/font/mod.rs`
- Run:
  - `cargo test -p roastty emoji_presentation -- --nocapture --test-threads=1`
  - `cargo test -p roastty get_index_default_presentation_emoji -- --nocapture --test-threads=1`
  - `cargo test -p roastty get_presentation_emoji -- --nocapture --test-threads=1`
  - `cargo test -p roastty fallback_presentation_check -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/795-unicode-table-checklist-sync.md`
- Run:
  - `git diff --check`

The experiment passes if the docs stop claiming there are no Unicode tables at
all while still keeping both Unicode rows unchecked and explicitly leaving the
missing broader Unicode/uucode parity work open. It is Partial if only the
dependency row can be corrected. It fails if the "no tables" wording remains
accurate.

## Design Review

Codex's first design review found one blocking issue: the top-level `uucode`
dependency-policy bullet still said Roastty Unicode tables were generated from
the UCD and matched Ghostty's exact property semantics, which read like a
completion claim. The design was fixed by changing that bullet to say Unicode
tables should be generated from the UCD as each table is ported.

Codex re-reviewed the fixed design and found no blocking findings. The review
approved the scope because both checklist rows remain unchecked, existing
coverage is limited to `Emoji_Presentation`, and the missing broader Unicode
property, grapheme-break, width/wcwidth tables, and standalone `unicode/`
namespace remain explicit.

## Result

**Result:** Pass

The Issue 801 Unicode/uucode rows no longer claim there are no Unicode tables at
all. The README now records the current scoped coverage:

- `roastty/src/font/emoji_presentation.rs` contains a generated
  `Emoji_Presentation` table from `vendor/uucode/ucd/emoji/emoji-data.txt`.
- `roastty/src/font/codepoint_resolver.rs` consults that table when defaulting
  presentation-less codepoints to emoji or text presentation.
- The top-level `uucode` policy bullet now describes the intended generation
  approach as each table is ported, rather than implying all tables already
  exist.

Both rows remain unchecked because Roastty still lacks a complete standalone
`unicode/` namespace and the broader Unicode property, grapheme-break, and
width/wcwidth table set remains open.

Verification:

- Inspected:
  - `roastty/src/font/emoji_presentation.rs`
  - `roastty/src/font/codepoint_resolver.rs`
  - `roastty/src/font/mod.rs`
- `cargo test -p roastty emoji_presentation -- --nocapture --test-threads=1` — 3
  passed
- `cargo test -p roastty get_index_default_presentation_emoji -- --nocapture --test-threads=1`
  — 1 passed
- `cargo test -p roastty get_presentation_emoji -- --nocapture --test-threads=1`
  — 1 passed
- `cargo test -p roastty fallback_presentation_check -- --nocapture --test-threads=1`
  — 1 passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/795-unicode-table-checklist-sync.md`
  — passed
- `git diff --check` — passed

## Conclusion

The uucode work is no longer accurately described as "not started." Roastty has
one generated UCD-backed table in active resolver use, but the dependency
remains partial until the rest of Ghostty's Unicode property, grapheme-break,
and width/wcwidth tables are ported.

## Completion Review

Codex reviewed the completed experiment and found no blocking findings. The
review approved the result because the rows remain unchecked, the wording stays
limited to the generated `Emoji_Presentation` table, the missing broader
Unicode/uucode work remains explicit, and the verification evidence records the
targeted tests, Prettier, and `git diff --check`.
