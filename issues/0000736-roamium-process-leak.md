# Issue 736: Roamium process leak on board crash

## Goal

When a board (Ghostboard or Wezboard) crashes, Roamium processes should detect
the dead connection and exit automatically instead of living forever as orphans.

## Background

Both Ghostboard and Wezboard manage Roamium's lifecycle. They spawn Roamium
processes, track them via `Child` process handles, and send a `Shutdown`
protobuf message when the last pane using a server closes (`pane_count == 0`).
The board then calls `wait()` on the child process.

If a board crashes, this graceful shutdown never happens. Here's the sequence:

1. The OS closes the board's socket file descriptors automatically.
2. Roamium's reader thread (`roamium/src/ipc.rs:48`) sees EOF and returns.
3. The Chromium main loop (`ts_content_main`) keeps running indefinitely.
4. Roamium becomes an orphaned process with no parent connection.

The reader thread exiting does nothing to stop the Chromium event loop. There is
no heartbeat, no parent PID monitoring, and no signal handling that would cause
Roamium to exit when the board disappears.

### Current shutdown paths

| Path       | Trigger                          | Result                                       |
| ---------- | -------------------------------- | -------------------------------------------- |
| Graceful   | Board sends `Shutdown` message   | `ts_quit()` called, clean exit               |
| Socket EOF | Board closes socket (or crashes) | Reader thread exits, Chromium loop continues |
| Manual     | `killall roamium`                | Process killed externally                    |

The socket EOF path is the gap — it detects disconnection but doesn't act on it.

### Proposed solution

When Roamium's reader thread detects EOF (socket closed), it should call
`ts_quit()` to shut down the Chromium event loop, just like the `Shutdown`
message handler does. This way, any socket disconnection — whether from a
graceful close or a board crash — triggers a clean Roamium exit.

This is the simplest fix: no new protocol messages, no heartbeat timers, no
parent PID polling. The detection mechanism already exists (`Ok(0)` in
`reader_loop`); it just needs to call `ts_quit()` instead of silently returning.
