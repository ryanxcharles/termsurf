# ABI App Symbols

This artifact records Experiment 4's embedded ABI app-bridge audit. It focuses
on the ABI identifiers used by the pinned Ghostty macOS app sources and their
renamed Roastty equivalents.

## Commands

Run from the repo root:

```bash
perl -ne 'while(/\b(ghostty_[A-Za-z0-9_]+)\s*\(/g){print "$1\n"}' \
  vendor/ghostty/include/ghostty.h | sort -u \
  > /tmp/issue805-exp4-ghostty-header-fns.txt

perl -ne 'while(/\b(roastty_[A-Za-z0-9_]+)\s*\(/g){print "$1\n"}' \
  roastty/include/roastty.h | sort -u \
  > /tmp/issue805-exp4-roastty-header-fns.txt

sed 's/^ghostty_/roastty_/' /tmp/issue805-exp4-ghostty-header-fns.txt \
  > /tmp/issue805-exp4-ghostty-header-fns-mapped.txt

rg --no-filename -o '(ghostty|GHOSTTY)_[A-Za-z0-9_]+' \
  vendor/ghostty/macos/Sources | sort -u \
  > /tmp/issue805-exp4-ghostty-swift-symbols.txt

rg --no-filename -o '(roastty|ROASTTY)_[A-Za-z0-9_]+' \
  roastty/macos/Sources | sort -u \
  > /tmp/issue805-exp4-roastty-swift-symbols.txt

sed -e 's/^ghostty_/roastty_/' -e 's/^GHOSTTY_/ROASTTY_/' \
  /tmp/issue805-exp4-ghostty-swift-symbols.txt \
  > /tmp/issue805-exp4-ghostty-swift-symbols-mapped.txt

rg --no-filename -o '(roastty|ROASTTY)_[A-Za-z0-9_]+' \
  roastty/include/roastty.h | sort -u \
  > /tmp/issue805-exp4-roastty-header-identifiers.txt

perl -0ne 'while(/#\[no_mangle\]\s*pub\s+extern\s+"C"\s+fn\s+(roastty_[A-Za-z0-9_]+)/g){print "$1\n"}' \
  roastty/src/lib.rs | sort -u \
  > /tmp/issue805-exp4-roastty-exported-fns.txt
```

## Counts

| Inventory                                                                  | Count |
| -------------------------------------------------------------------------- | ----: |
| Upstream `ghostty.h` function declarations                                 |    90 |
| Roastty `roastty.h` function declarations                                  |   250 |
| Upstream Swift `ghostty_*` / `GHOSTTY_*` identifiers under `macos/Sources` |   538 |
| Roastty Swift `roastty_*` / `ROASTTY_*` identifiers under `macos/Sources`  |   539 |
| Roastty header identifiers                                                 |  1264 |
| Roastty exported `#[no_mangle] pub extern "C" fn` definitions              |   249 |

Evidence: `logs/issue805-exp4-abi-extraction.log`.

## Header Function Delta

After mapping upstream `ghostty_` function names to `roastty_`, the full header
function comparison reports these missing Roastty declarations:

| Mapped upstream function           | Classification                                                                                                                                                   | Evidence                                                                                                                                                                                                                  | Status                                                                                 |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `roastty_app_open_config`          | Upstream public function not referenced by `vendor/ghostty/macos/Sources`; not app-facing in this experiment.                                                    | `rg` finds `ghostty_app_open_config` in `vendor/ghostty/include/ghostty.h` and `vendor/ghostty/src/apprt/embedded.zig`, but not in `vendor/ghostty/macos/Sources`.                                                        | Not applicable to Experiment 4 app-facing slice.                                       |
| `roastty_benchmark_cli`            | Upstream public benchmark helper referenced by macOS tests, not app sources.                                                                                     | `vendor/ghostty/macos/Tests/BenchmarkTests.swift` references `ghostty_benchmark_cli`; `roastty/macos/Tests/BenchmarkTests.swift` references `roastty_benchmark_cli`, but `roastty/include/roastty.h` does not declare it. | Gap for a later macOS test/benchmark parity experiment, outside this app-source audit. |
| `roastty_inspector_metal_shutdown` | Upstream public function not referenced by `vendor/ghostty/macos/Sources`; inspector Swift sources use init/render/input/size/focus/text calls but not shutdown. | `rg` finds `ghostty_inspector_metal_shutdown` in `vendor/ghostty/include/ghostty.h` and `vendor/ghostty/src/apprt/embedded.zig`, but not in `vendor/ghostty/macos/Sources`.                                               | Not applicable to Experiment 4 app-facing slice.                                       |
| `roastty_translate`                | Upstream public localization helper not referenced by `vendor/ghostty/macos/Sources`.                                                                            | `rg` finds `ghostty_translate` in `vendor/ghostty/include/ghostty.h` and `vendor/ghostty/src/main_c.zig`, but not in `vendor/ghostty/macos/Sources`.                                                                      | Not applicable to Experiment 4 app-facing slice.                                       |

Experiment 5 resolved this full-header delta. The same mapped header comparison
now reports no missing Roastty declarations, and the header/export comparison
reports only the known `roastty_string_s` callback typedef false positive.

| Mapped upstream function           | Experiment 5 outcome                                                                                                                                 | Evidence                                                                                                                                                                                                                                | Status                 |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `roastty_app_open_config`          | Declared and exported; dispatches `ROASTTY_ACTION_OPEN_CONFIG` through the app runtime action callback.                                              | `roastty/include/roastty.h`; `roastty/src/lib.rs`; `roastty/tests/abi_harness.c`; `logs/issue805-exp5-static-abi.log`; `logs/issue805-exp5-cargo-test-roastty.log`.                                                                     | Pass                   |
| `roastty_benchmark_cli`            | Declared and exported; intentionally returns `false` because Roastty has no benchmark CLI port.                                                      | `roastty/include/roastty.h`; `roastty/src/lib.rs`; `roastty/tests/abi_harness.c`; `roastty/macos/Tests/BenchmarkTests.swift`; `logs/issue805-exp5-cargo-test-roastty.log`; `issues/0805-roastty-ghostty-parity/divergences.md#div-002`. | Intentional divergence |
| `roastty_inspector_metal_shutdown` | Declared and exported; returns `true` for a valid inspector handle and `false` for null while Roastty's inspector Metal backend remains unsupported. | `roastty/include/roastty.h`; `roastty/src/lib.rs`; `roastty/tests/abi_harness.c`; `logs/issue805-exp5-cargo-test-roastty.log`.                                                                                                          | Pass                   |
| `roastty_translate`                | Declared and exported; intentionally returns the input pointer unchanged because Roastty has no catalog-backed translation runtime.                  | `roastty/include/roastty.h`; `roastty/src/lib.rs`; `roastty/src/os/i18n.rs`; `roastty/tests/abi_harness.c`; `logs/issue805-exp5-cargo-test-roastty.log`; `issues/0805-roastty-ghostty-parity/divergences.md#div-001`.                   | Intentional divergence |

## Swift Symbol Delta

After mapping upstream Swift identifiers with `ghostty_ -> roastty_` and
`GHOSTTY_ -> ROASTTY_`:

- Missing mapped Roastty Swift identifiers: none.
- Extra Roastty Swift identifiers: `ROASTTY_UI_KEY_TRACE_PATH`.

`ROASTTY_UI_KEY_TRACE_PATH` is a Roastty-only GUI automation trace environment
key used by prior input experiments. It is not a Ghostty ABI requirement and
does not affect upstream app-facing symbol parity.

## Swift-Used Type and Constant Declarations

The raw declaration check compares mapped Swift identifiers to identifiers
declared in `roastty/include/roastty.h`. It reports:

| Identifier                    | Classification                                                                                | Evidence                                                                                                   | Status                                         |
| ----------------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `ROASTTY_CLEAR_USER_DEFAULTS` | Environment variable string used by AppDelegate, not a C ABI declaration.                     | Ghostty and Roastty both read the renamed environment variable from `ProcessInfo.processInfo.environment`. | Not applicable to C header declaration parity. |
| `ROASTTY_CONFIG_PATH`         | Environment variable string used by AppDelegate, not a C ABI declaration.                     | Ghostty and Roastty both read the renamed environment variable from `ProcessInfo.processInfo.environment`. | Not applicable to C header declaration parity. |
| `ROASTTY_MAC_LAUNCH_SOURCE`   | Environment variable string used by package launch-source detection, not a C ABI declaration. | Ghostty and Roastty both read the renamed environment variable from `ProcessInfo.processInfo.environment`. | Not applicable to C header declaration parity. |
| `ROASTTY_QUICK_TERMINAL`      | Environment variable string passed into terminal config, not a C ABI declaration.             | Ghostty and Roastty both set the renamed environment variable in Quick Terminal code.                      | Not applicable to C header declaration parity. |
| `ROASTTY_USER_DEFAULTS_SUITE` | Environment variable string used by UserDefaults helper, not a C ABI declaration.             | Ghostty and Roastty both read the renamed environment variable from `ProcessInfo.processInfo.environment`. | Not applicable to C header declaration parity. |
| `roastty_app`                 | Swift local/property name, not a C ABI identifier.                                            | `roastty_app` appears as Swift state/local variables in iOS and terminal controller sources.               | Not applicable to C header declaration parity. |

All Swift-used C ABI functions, typedefs, structs, enums, enum values, macros,
and constants have mapped Roastty declarations in `roastty/include/roastty.h`.
The declaration check does not report missing app-facing ABI declarations.

## Implementation Export Delta

The export check compares Roastty header function declarations to
`#[no_mangle] pub extern "C" fn` definitions in `roastty/src/lib.rs`.

Raw missing export:

- `roastty_string_s`

Classification: false positive. The header extractor matches callback typedef
return types such as `typedef roastty_string_s (*roastty_terminal_enquiry_cb)(`
because they look like function declarations to the simple regex. There is no
`roastty_string_s(...)` function declaration in `roastty/include/roastty.h`, so
this is not a missing export.

## Conclusion

The app-facing embedded ABI bridge passes this audit slice:

- every upstream Swift app-source identifier has a mapped Roastty Swift
  identifier;
- every mapped Swift-used C ABI identifier is declared in
  `roastty/include/roastty.h`;
- app-facing header functions are implemented as exported
  `#[no_mangle] pub extern "C" fn` definitions or otherwise classified;
- extra Roastty symbols are either expected Roastty-only support or outside the
  app-facing ABI surface.

Experiment 5 later resolved the full Ghostty header delta recorded here. This
artifact still represents Experiment 4's app-facing audit, but the non-app
functions are no longer undeclared: `roastty_app_open_config`,
`roastty_benchmark_cli`, `roastty_inspector_metal_shutdown`, and
`roastty_translate` now have Roastty declarations and exports.

The macOS benchmark CLI remains an accepted semantic divergence, not a missing
symbol gap: `roastty_benchmark_cli` is declared/exported for ABI and copied-test
link safety, but it returns `false` because Roastty has no benchmark CLI port.
See `DIV-002` in `divergences.md`.
