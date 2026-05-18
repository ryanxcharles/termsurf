---
name: debug-logs
description: "Store all debug logs in ~/dev/termsurf/logs/. Use when running apps, configuring log paths, or troubleshooting output."
---

# Debug Logs

All debug logs go in `~/dev/termsurf/logs/`. This directory is gitignored.
Never write logs to `/tmp/` or any other location outside the repo.

## Log Directory

```
~/dev/termsurf/logs/
```

Create it if it doesn't exist:

```bash
mkdir -p ~/dev/termsurf/logs
```

## Naming Convention

Log files are named `<app-name>.log`:

| App | Log file |
|-----|----------|
| Two Profiles receiver | `two-profiles-receiver.log` |
| One Profile app | `one-profile.log` |
| Box demo server | `box-demo.log` |

## Per-App-Type Redirection

### Launchd plists

Set `StandardOutPath` and `StandardErrorPath` in the plist XML:

```xml
<key>StandardOutPath</key>
<string>/Users/ryan/dev/termsurf/logs/<name>.log</string>
<key>StandardErrorPath</key>
<string>/Users/ryan/dev/termsurf/logs/<name>.log</string>
```

### macOS .app bundles (C++, Swift)

```bash
open "App Name.app" \
  --stdout ~/dev/termsurf/logs/<name>.log \
  --stderr ~/dev/termsurf/logs/<name>.log \
  --args <arguments>
```

### CLI binaries (C++, Rust, Objective-C)

```bash
./binary args > ~/dev/termsurf/logs/<name>.log 2>&1
```

Or to see output live while also logging:

```bash
./binary args 2>&1 | tee ~/dev/termsurf/logs/<name>.log
```

### Bun / TypeScript

```bash
bun run server.ts > ~/dev/termsurf/logs/<name>.log 2>&1
```
