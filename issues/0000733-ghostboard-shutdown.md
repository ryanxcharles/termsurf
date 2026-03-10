# Issue 733: Ghostboard should send Shutdown instead of SIGKILL

## Goal

Ghostboard should use the new `Shutdown` protocol message to gracefully
terminate Roamium, instead of force-killing it with SIGKILL.

## Background

Issue 732 added a `Shutdown` message to the TermSurf protocol and removed
Roamium's self-termination on last tab close. Roamium now waits for an explicit
`Shutdown` message before exiting. Wezboard already sends `Shutdown` when
`pane_count` drops to 0.

Ghostboard still works — when `pane_count` reaches 0, it calls `killServer()`
(`xpc.zig:1042`), which sends SIGKILL via `proc.kill()`. SIGKILL terminates the
process, so the behavior is correct but not graceful. Chromium's cleanup code
(atexit handlers, shared memory cleanup, IPC channel teardown) never runs.

### Current Ghostboard code path

In `handleClientDisconnect()` (`xpc.zig:1724-1757`), when a TUI disconnects:

1. Sends `CloseTab` to Roamium for each pane (`msg_case = 4`, line 1731)
2. Decrements `server.pane_count` (line 1736)
3. If `pane_count == 0`:
   - Calls `killServer(server)` — sends SIGKILL, then `proc.wait()` (line 1738)
   - Cancels Chromium's dispatch source and closes its socket fd (lines
     1743-1751)
   - Removes server from `servers` map and frees memory (lines 1753-1756)

### What should change

Replace `killServer(server)` with:

1. Send `Shutdown` message to Roamium via `sendProtobuf(server.fd, &wrapper)`
   (msg_case = 31)
2. Wait for Roamium to exit via `proc.wait()` — Roamium calls `ts_quit()` on
   receiving Shutdown, which exits quickly
3. Keep `killServer()` for app-exit cleanup (line 173-182) as a safety net

The rest of the cleanup (cancel dispatch source, remove server entry, free
memory) stays the same.

### Relevant code

| Location          | What it does                                             |
| ----------------- | -------------------------------------------------------- |
| `xpc.zig:1042-49` | `killServer()` — SIGKILL + wait                          |
| `xpc.zig:1724-57` | TUI disconnect: CloseTab, decrement, kill when count = 0 |
| `xpc.zig:173-182` | App exit cleanup: kills all remaining servers            |
| `xpc.zig:2303-24` | `sendProtobuf()` — send length-prefixed protobuf         |

### Protobuf details

The `Shutdown` message was added in Issue 732 as field 31 in the `oneof msg`
block. In Ghostboard's protobuf-c bindings, this corresponds to `msg_case = 31`.
The message body is empty — it's a signal with no payload. Need to check how
protobuf-c handles the new field (the `.c`/`.h` files may need regeneration from
`termsurf.proto`).
