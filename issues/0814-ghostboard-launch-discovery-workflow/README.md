+++
status = "closed"
opened = "2026-06-17"
closed = "2026-06-17"
+++

# Issue 814: Ghostboard Launch and Browser Discovery Workflow

## Goal

Harden Ghostboard launch, socket discovery, debug-vs-installed binary selection,
and named/default browser startup.

## Background

Issue 810 classified this as a `Highly likely` workflow gap. The audit found
risk around named/default browser launch, installed-vs-debug binary selection,
socket discovery, app identity, and config paths. In particular, current
Ghostboard evidence indicated absolute browser paths can spawn, while named
browser launch such as default `roamium` is incomplete.

## Analysis

This issue should make the ordinary developer and app launch paths explicit and
reliable. The expected behavior should be documented before fixes so tests can
distinguish debug binaries, installed binaries, default browser names, and
explicit absolute browser paths.

Verification should include:

- debug Ghostboard can launch debug Roamium through an explicit path;
- default or named `roamium` resolves correctly where supported;
- webtui discovers `TERMSURF_SOCKET` under normal app launch;
- stale installed binaries are not accidentally used during debug testing;
- failure messages are clear when a browser cannot be resolved.

The debug launch/discovery contract is documented in
[Ghostboard Launch Discovery](../../docs/ghostboard-launch-discovery.md).

## Experiments

- [Experiment 1: Resolve named Roamium for debug launch](01-resolve-named-roamium-debug-launch.md)
  — **Pass**
- [Experiment 2: Document launch discovery contract](02-document-launch-discovery-contract.md)
  — **Pass**

## Conclusion

Issue 814 is closed. Ghostboard now has a deterministic debug launch workflow:
absolute browser paths still spawn exactly as provided, default/named `roamium`
resolves through an explicit absolute `TERMSURF_ROAMIUM_PATH`, invalid named
browser configuration fails clearly before creating a pending server, and the
debug harness proves the contract without silently using stale installed Roamium
paths.

The launch/discovery contract is documented in
[Ghostboard Launch Discovery](../../docs/ghostboard-launch-discovery.md), and
`scripts/ghostboard-geometry-matrix.sh launch-discovery-contract` provides a
cheap regression check for the command/environment rules. Normal installed app
identity and installed Roamium discovery remain intentionally deferred to
Issue 819.
