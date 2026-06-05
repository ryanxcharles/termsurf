+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 535: the CLI-args driver (Config::set_cli_args)

## Description

With the per-arg parser (`parse_cli_arg`, Experiment 534) and `Config::set` (43
of 44 fields), this experiment ports the multi-arg **CLI driver** —
`Config::set_cli_args` — the CLI counterpart to `Config::load_str` (Experiment
529). It iterates the arguments, applies each `--key=value` via `Config::set`,
records an "invalid field" diagnostic for a non-flag argument, and collects
per-arg diagnostics (continuing rather than aborting). This is the last
config-source driver.

## Upstream behavior

Upstream `cli.args.parse` (`cli/args.zig:55`) iterates the args:

- a non-`--` argument is **not a config flag** — it appends a diagnostic
  (`key = arg`, message `"invalid field"`, the iterator's location) and
  continues.
- otherwise `parse_cli_arg` extracts `(key, value)` and `parseIntoField` is
  called; on error it appends a diagnostic and continues.

So loading the CLI args is: for each arg (positionally), `parse_cli_arg`; if it
yields `(key, value)`, `Config::set(key, value)`, recording a diagnostic on
error; if it is a non-flag arg, record an "invalid field" diagnostic. The loader
never aborts on a bad arg.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
impl Config {
    /// Apply config from CLI arguments (upstream `cli.args.parse` over args): for each
    /// argument, parse the `--key=value` form (`parse_cli_arg`) and apply it via
    /// `Config::set`; a non-flag argument or a `Config::set` error records a
    /// diagnostic, and the loop continues. The diagnostic's `line` is the 1-based
    /// argument position.
    pub(crate) fn set_cli_args<'a, I>(&mut self, args: I) -> Vec<ConfigDiagnostic>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut diagnostics = Vec::new();
        for (i, arg) in args.into_iter().enumerate() {
            match loader::parse_cli_arg(arg) {
                Some((key, value)) => {
                    if let Err(error) = self.set(key, value) {
                        diagnostics.push(ConfigDiagnostic { line: i + 1, key: key.to_string(), error });
                    }
                }
                // A non-flag argument is not a valid config field.
                None => diagnostics.push(ConfigDiagnostic {
                    line: i + 1,
                    key: arg.to_string(),
                    error: ConfigSetError::UnknownField,
                }),
            }
        }
        diagnostics
    }
}
```

Each argument is `parse_cli_arg`-parsed; a `--key=value` arg drives
`Config::set` (recording a diagnostic on a field error); a non-flag arg records
an `UnknownField`-kind diagnostic (the roastty analog of upstream's "invalid
field"). The loop continues past errors; `ConfigDiagnostic.line` carries the
**1-based argument position** (the CLI analog of a file line, reusing the
existing diagnostic type).

## Scope / faithfulness notes

- **Ported (bridged)**: the multi-arg CLI driver of `cli.args.parse`, as
  `Config::set_cli_args`.
- **Faithful**: per-arg iteration; `--key=value` ⇒ `Config::set`; a non-flag arg
  ⇒ an "invalid field" diagnostic; **continue past errors**, collecting a
  diagnostic per failing arg — matching upstream's `parse` (record + continue).
- **Faithful adaptation**: upstream's iterator + `Location` → iterating the args
  with `enumerate()` and reusing `ConfigDiagnostic` with `line` = the 1-based
  argument position; upstream's "invalid field" message for a non-flag arg →
  `ConfigSetError::UnknownField` with the arg as the key (roastty's coarser
  error model, the same "not a valid field" outcome); the `parseManuallyHook` /
  `--help` / `compatibility` hooks are N-A for roastty config.
- **Input contract**: `set_cli_args` receives the **config** arguments.
  Upstream's outer process-args wrapper skips action arguments beginning with
  `+` before the config `parse` sees them; that `+`-arg filtering is a separate
  outer layer (not this driver), so a `+action` passed here would be reported as
  an invalid field.
- **Deferred**: the `loadDefaultFiles` orchestration (pending roastty's config
  naming); a source-aware diagnostic `Location` (file-line vs CLI-arg) — `line`
  doubles as both. `background-image-opacity` stays float-blocked.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `Config::set_cli_args`.
2. Tests (in `config/mod.rs`): a list of `--key=value` args applies each field
   (verified via `format_config`) with no diagnostics; a bare-flag arg
   (`--background-image-repeat` ⇒ `true`); a non-flag arg and an invalid field
   record diagnostics with the correct 1-based positions while the other args
   still apply (continue past errors).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config_set_cli_args
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Config::set_cli_args` applies each `--key=value` arg via `Config::set`,
  records an "invalid field" diagnostic for a non-flag arg, and collects a
  diagnostic per failing arg (1-based position) while continuing — faithful to
  upstream's `parse`;
- the tests pass (a clean args apply + an apply with errors and correct
  positions), and the existing tests still pass;
- the `loadDefaultFiles` orchestration stays deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the driver diverges from upstream (esp. aborting on
an error or mis-positioning diagnostics), an unrelated item changes, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Low** finding (folded into the scope notes): document the `+…` action-arg
behavior — upstream's process-args wrapper increments the CLI index but skips
args beginning with `+` before `parse` sees them (`args.zig:1322`/`:1346`);
`set_cli_args` receives the already-filtered **config** args, so the `+`-arg
filtering is a separate outer layer (a `+action` passed here would be reported
as an invalid field).

Codex found everything else faithful: reusing `ConfigDiagnostic.line` as a
1-based CLI argument position is acceptable for this coarser diagnostic model
(upstream's CLI location is also 1-indexed — `index` starts at 0 and increments
before yielding, `args.zig:1335`/`:1359`); a source-aware location type would be
more precise but is not required for this slice; the non-`--` path recording
`key = arg` with a coarse `UnknownField`/invalid-field diagnostic is an
acceptable narrowing of upstream's distinct `"invalid field"` message
(`args.zig:109`); and continue-past-errors is faithful — upstream appends
diagnostics and continues for both non-flags and parse errors
(`args.zig:115`/`:173`).

Review artifacts:

- Prompt: `logs/codex-review/20260604-193048-d535-prompt.md` (design)
- Result: `logs/codex-review/20260604-193048-d535-last-message.md` (design)
