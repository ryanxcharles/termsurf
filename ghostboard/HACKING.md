# Developing Ghostboard

Ghostboard is TermSurf's Ghostty-based terminal GUI. The code is kept close to
upstream Ghostty so that upstream releases can be compared and merged cleanly,
while TermSurf-specific work is tracked by repository issues.

## Build Commands

Use the selected full Xcode installation on macOS. For normal Zig builds:

```shell
zig build
```

Useful targets:

| Command                                               | Description                                |
| ----------------------------------------------------- | ------------------------------------------ |
| `zig build run`                                       | Build and run the terminal                 |
| `zig build test`                                      | Run unit tests                             |
| `zig build test -Dtest-filter=<name>`                 | Run matching unit tests                    |
| `zig build -Demit-macos-app=false`                    | Rebuild the Zig core without the macOS app |
| `macos/build.nu --configuration Debug --action build` | Build the macOS app bundle                 |

The macOS app output is under `macos/build/<configuration>/TermSurf.app`.

## Agents

Use the root TermSurf instructions in `../AGENTS.md` for issue workflow,
reviews, commits, and repository-wide rules. Local `AGENTS.md` files under this
directory provide only build and subsystem guidance.

Do not apply upstream Ghostty contribution-policy automation to TermSurf work.

## Logs

For TermSurf debugging, store logs under `../logs/` unless a test or build tool
requires another location.
