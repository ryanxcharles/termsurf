# Experiment 117: Scrollback Limit Runtime Split

## Description

`RUNTIME-009B` currently groups several terminal runtime effects together:
scrollback, alternate screen, shell integration, terminfo, title reporting, and
remaining terminal behavior toggles. Inspection after Experiment 116 found a
concrete missing runtime wire: Roastty parses `scrollback-limit`, but PTY-backed
surface startup does not pass that parsed config into `TermioSpawnOptions`.
`Termio::spawn_with_options` initializes its terminal with `None` for the
scrollback limit, so config-level `scrollback-limit = 0` cannot currently
disable scrollback for new surfaces.

Pinned Ghostty documents `scrollback-limit` as a per-surface limit that affects
new terminal surfaces. Ghostty's terminal init receives
`full_config.scrollback-limit`. Roastty's internal terminal API currently
accepts a row-based `max_scrollback_rows`, not Ghostty's byte-accurate memory
limit, so this experiment will prove a narrow, useful runtime slice rather than
claim full byte-quota parity.

This experiment will:

- wire parsed config `scrollback-limit = 0` into new PTY-backed surfaces as "no
  scrollback";
- preserve the existing default/nonzero behavior as allowing scrollback;
- keep exact byte-quota parity for nonzero `scrollback-limit` values in the
  remaining gap;
- split the already-proven alternate-screen no-scrollback terminal-core behavior
  out of the broad terminal row without claiming app/GUI behavior.

The intended inventory result is:

- `RUNTIME-009B1`: `Oracle complete` for parsed config `scrollback-limit = 0`
  disabling PTY-backed surface history, default/nonzero scrollback still
  allowing history, and terminal-core alternate screen having no scrollback.
- `RUNTIME-009B2`: `Gap` for exact nonzero `scrollback-limit` byte quota parity,
  shell integration, terminfo, title reporting, and remaining terminal behavior
  effects.

## Changes

- `roastty/src/termio.rs`
  - Add a `max_scrollback_rows` option to `TermioSpawnOptions`.
  - Pass that option into `Terminal::init_with_options`.
  - Keep the default as `None` so existing tests and nonzero/default surface
    behavior continue to allow scrollback.
- `roastty/src/lib.rs`
  - Convert parsed config `scrollback-limit = 0` into `Some(0)` for
    `TermioSpawnOptions`.
  - Leave nonzero values mapped to `None` for now, and document that exact
    byte-quota parity remains in the next terminal gap row.
  - Add focused tests for:
    - parsed config `scrollback-limit = 0` disables scrollback rows on a
      PTY-backed surface;
    - default parsed config allows scrollback rows on the same PTY-backed
      surface scenario;
    - the existing alternate-screen no-scrollback terminal-core guard remains
      passing.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Replace broad `RUNTIME-009B` with narrower `RUNTIME-009B1` and
    `RUNTIME-009B2` rows.
  - Update `EXPECTED_IDS`.
  - Mark `RUNTIME-009B1` `Oracle complete` only with evidence from the new
    parsed-config scrollback-limit runtime tests and existing alternate-screen
    terminal-core guard.
  - Keep `RUNTIME-009B2` as `Gap` with explicit missing evidence for exact
    scrollback byte quota, shell integration, terminfo, title reporting, and
    remaining terminal behavior.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate via `config_runtime_inventory.py` so `CFG-223` reflects the new
    row counts.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning that `scrollback-limit` parity currently has two tiers:
    proven zero/no-history behavior and unproven exact nonzero byte-quota
    behavior.
  - Update the experiment index as the result is recorded.

## Verification

Pass criteria:

- The focused runtime tests pass:

  ```sh
  cargo test --manifest-path roastty/Cargo.toml config_scrollback_limit_runtime
  cargo test --manifest-path roastty/Cargo.toml terminal_stream_alt_screen_has_no_scrollback_and_formatter_reads_active_screen
  ```

- Rust formatting passes:

  ```sh
  cargo fmt --manifest-path roastty/Cargo.toml -- --check
  ```

- The runtime inventory validates the new manifest and reports the expected row
  split:

  ```sh
  PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py \
    --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
    --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
  ```

- A matrix assertion proves:
  - old `RUNTIME-009B` is absent;
  - `RUNTIME-009B1` is `Oracle complete`;
  - `RUNTIME-009B1` evidence or guard cells name
    `config_scrollback_limit_runtime` and the alternate-screen no-scrollback
    guard;
  - `RUNTIME-009B1` missing evidence starts with `None`;
  - `RUNTIME-009B2` remains `Gap`;
  - `RUNTIME-009B2` retains exact nonzero scrollback byte quota, shell
    integration, terminfo, title reporting, and remaining terminal behavior;
  - `CFG-223` remains `Gap`.

  ```sh
  PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
  from pathlib import Path

  inventory = Path("issues/0805-roastty-ghostty-parity/config-runtime-inventory.md").read_text()
  matrix = Path("issues/0805-roastty-ghostty-parity/config-matrix.md").read_text()

  rows = {}
  for line in inventory.splitlines():
      if not line.startswith("| RUNTIME-"):
          continue
      cells = [cell.strip() for cell in line.strip("|").split("|")]
      rows[cells[0]] = cells

  assert "RUNTIME-009B" not in rows, rows.get("RUNTIME-009B")
  assert len(rows) == 26, len(rows)
  assert rows["RUNTIME-009B1"][5] == "Oracle complete", rows["RUNTIME-009B1"]
  for term in (
      "config_scrollback_limit_runtime",
      "scrollback-limit = 0",
      "alternate-screen no-scrollback",
  ):
      assert term in rows["RUNTIME-009B1"][6] or term in rows["RUNTIME-009B1"][9], (
          term,
          rows["RUNTIME-009B1"],
      )
  assert rows["RUNTIME-009B1"][7].startswith("None"), rows["RUNTIME-009B1"]
  assert rows["RUNTIME-009B2"][5] == "Gap", rows["RUNTIME-009B2"]
  behavior = rows["RUNTIME-009B2"][1]
  for term in ("byte quota", "shell integration", "terminfo", "title reporting", "remaining terminal"):
      assert term in behavior, (term, rows["RUNTIME-009B2"])
  cfg223 = next(line for line in matrix.splitlines() if line.startswith("| CFG-223 "))
  assert "| Gap " in cfg223, cfg223
  PY
  ```

- Markdown and diff hygiene pass:

  ```sh
  prettier --check issues/0805-roastty-ghostty-parity/README.md \
    issues/0805-roastty-ghostty-parity/117-scrollback-limit-runtime-split.md \
    issues/0805-roastty-ghostty-parity/config-matrix.md \
    issues/0805-roastty-ghostty-parity/config-runtime-inventory.md

  git diff --check
  ```

## Design Review

Adversarial review: **Approved**.

The reviewer confirmed the README links Experiment 117 as `Designed`, the
required sections are present, the source and upstream claims are grounded, and
the verification criteria cover the scoped runtime behavior. No required
findings were reported.
