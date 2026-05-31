# Experiment 11: Add WebAuthn Virtual-Authenticator Coverage

## Description

Experiment 10 left one intentionally unresolved harness classification:

```text
logs/issue-799-browser-api-audit/20260531-025116-412177
webauthn-create: blocked_needs_virtual_authenticator
missing_interfaces: []
empty_interfaces: []
```

The `webauthn-create` probe already reaches the WebAuthn runtime without a
missing binder or bad Mojo crash, but it cannot prove the browser-service path
because no authenticator exists. Experiment 1 classified this exact gap as
`Automatable after setup`: use a DevTools virtual authenticator and a local
WebAuthn fixture to prove the contained path.

Roamium already prints a DevTools websocket endpoint for each harness probe:

```text
DevTools listening on ws://127.0.0.1:<port>/devtools/browser/<id>
```

That means this experiment should not add native WebAuthn UI, OS authenticator
integration, or a TermSurf credential manager product feature. It should add
only the automation setup needed to prove that a WebAuthn page can complete
against Chromium's DevTools virtual authenticator without renderer kills,
missing binders, or manual interaction.

Real passkey/security-key UX remains deferred.

## Changes

1. Add a small DevTools Protocol helper for the harness.

   Add a script such as:

   ```text
   scripts/issue-799-webauthn-virtual-authenticator.mjs
   ```

   The helper should:
   - accept `--devtools-port`, `--url-contains`, `--out`, and
     `--timeout-seconds`;
   - poll `http://127.0.0.1:<port>/json/list` for the page target whose URL
     contains the `webauthn-create` probe URL;
   - require the selected target to have `type == "page"` and a URL containing
     `/probe/webauthn-create.html`;
   - connect to the target's `webSocketDebuggerUrl`;
   - send `WebAuthn.enable`;
   - send `WebAuthn.addVirtualAuthenticator` with deterministic options:

   ```json
   {
     "options": {
       "protocol": "ctap2",
       "transport": "usb",
       "hasResidentKey": true,
       "hasUserVerification": true,
       "isUserVerified": true,
       "automaticPresenceSimulation": true
     }
   }
   ```

   - write a JSON artifact containing target-selection evidence (`id`, `type`,
     `url`, and websocket endpoint path), whether `WebAuthn.enable` succeeded,
     the returned `authenticatorId`, and any DevTools error.

   Reuse the existing DevTools helper style from scripts such as
   `capture-devtools-screenshot.mjs` and `probe-pdf-toolbar.mjs`. Do not add a
   new npm dependency unless the existing Node runtime lacks a global
   `WebSocket`.

2. Make the WebAuthn page probe activation-controlled.

   Update the `webauthn-create` probe in
   `scripts/test-issue-799-browser-api-audit.py` so it behaves like the File
   System Access probe:
   - render a visible button;
   - report `ready`;
   - wait for the harness to send synthetic mouse activation;
   - report `activated`;
   - call `navigator.credentials.create({ publicKey: ... })` from the click
     handler;
   - return a deterministic final status.

   The final statuses should distinguish:
   - `webauthn_virtual_authenticator_created` — credential creation completed
     against the virtual authenticator;
   - `blocked_user_activation` — the click/activation path failed;
   - `blocked_needs_virtual_authenticator` — no virtual authenticator was
     installed or Chromium did not route to it before timeout;
   - `rejected` — WebAuthn rejected for another explicit reason;
   - `unsupported` — the API surface is absent.

   A successful final report must include concrete credential evidence from the
   returned `PublicKeyCredential`, not only a status label:
   - `credential.type == "public-key"`;
   - nonempty `credential.id`;
   - `credential.rawId.byteLength > 0`;
   - `credential.response.attestationObject.byteLength > 0`;
   - `credential.response.clientDataJSON.byteLength > 0`.

   The verifier should reject `webauthn_virtual_authenticator_created` if any of
   these fields are absent or zero-sized. This proves Chromium's WebAuthn stack
   returned a real virtual-authenticator credential without claiming real
   passkey/security-key product support.

3. Teach the Python harness to install the virtual authenticator.

   In `run_probe()`, when `probe.name == "webauthn-create"`:
   - discover the DevTools port by tailing the probe's `roamium.stderr` for
     `DevTools listening on ws://127.0.0.1:<port>/...`;
   - wait until the page has reported `ready`;
   - run the DevTools helper against the probe page target;
   - record helper output under the probe directory;
   - only after the helper reports an `authenticatorId`, send the same contained
     mouse activation used by the File System Access probe.

   If the DevTools endpoint never appears, classify the probe as a harness setup
   failure, not as browser API success.

4. Add WebAuthn-specific verification/classification.

   Add a verifier analogous to the File System Access verifier. It should
   require:
   - `devtools_port` discovered;
   - `WebAuthn.enable` succeeded;
   - `authenticatorId` returned;
   - DevTools target evidence shows `type == "page"` and a URL containing
     `/probe/webauthn-create.html`;
   - activation was sent and observed;
   - the page reported `webauthn_virtual_authenticator_created`;
   - the page reported concrete `PublicKeyCredential` material:
     `type == "public-key"`, nonempty `id`, nonzero `rawId.byteLength`, nonzero
     `attestationObject.byteLength`, and nonzero `clientDataJSON.byteLength`;
   - no missing binder, bad Mojo, or unexpected crash appeared.

   The resulting classification should be named something explicit, such as:

   ```text
   webauthn_virtual_authenticator_completed
   ```

   Keep `blocked_needs_virtual_authenticator` as a failure for this experiment.

5. Update coverage maps.

   Update `coverage-map.md` and `reference-coverage-map.md` generation so the
   new classification explains that the virtual-authenticator path is covered,
   while real WebAuthn/passkey UX remains deferred.

6. Do not change product WebAuthn behavior.

   This experiment is harness/automation coverage unless Chromium proves a small
   TermSurf embedder hook is missing. Do not add:
   - native passkey/security-key UI;
   - persistent credential storage;
   - protocol prompt/reply messages;
   - OS authenticator integration;
   - Chrome's full WebAuthn product stack.

7. If a Chromium-side hook is unexpectedly required, stop and redesign.

   The expected implementation is harness-only. If the virtual authenticator
   cannot work because TermSurf lacks a Chromium embedder hook, record the exact
   missing hook and close this experiment as `Partial`; do not silently expand
   into product WebAuthn support.

## Verification

1. Run the focused WebAuthn probe:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py --probe webauthn-create
   ```

   Pass criteria:
   - run status is `completed`;
   - classification is `webauthn_virtual_authenticator_completed`;
   - the probe result records a DevTools port, selected page target evidence,
     and a virtual `authenticatorId`;
   - activation was sent and observed;
   - the page reports a completed credential creation result with
     `PublicKeyCredential` evidence: type `public-key`, nonempty `id`, nonzero
     `rawId`, nonzero attestation object bytes, and nonzero client data bytes;
   - `missing_interfaces` is empty;
   - `empty_interfaces` is empty;
   - no bad-Mojo or unexpected crash signature appears.

2. Run the Experiment 10 focused permission regression set:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py \
     --probe permissions-query \
     --probe geolocation-deny \
     --probe notification-permission \
     --probe file-system-access \
     --probe service-worker-basic
   ```

   Pass criteria:
   - `permissions-query`, `geolocation-deny`, and `notification-permission`
     remain `default_denied`;
   - `file-system-access` remains `file_system_access_denied`;
   - `service-worker-basic` remains cleanly exercised;
   - no missing binder, empty binder, bad Mojo, or unexpected crash appears.

3. Run the full Issue 799 harness:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py
   ```

   Pass criteria:
   - overall status is `completed`;
   - `missing_interfaces` is empty;
   - `empty_interfaces` is empty;
   - `webauthn-create` is `webauthn_virtual_authenticator_completed`;
   - previous completed probes remain green, including
     `renderer-crash-recovery`, JavaScript dialogs, downloads, page zoom,
     console capture, HTTP auth, default-deny permissions, and File System
     Access.

4. Run syntax checks:

   ```bash
   python3 -m py_compile scripts/test-issue-799-browser-api-audit.py
   node --check scripts/issue-799-webauthn-virtual-authenticator.mjs
   git diff --check
   ```

   If any Rust or Chromium code is edited unexpectedly, also run the required
   formatter/build gates for that language.

## Failure Criteria

The experiment fails if:

- `webauthn-create` remains `blocked_needs_virtual_authenticator`;
- the probe passes only because it times out or suppresses the WebAuthn call;
- the page can create a credential only with manual interaction;
- the helper connects to the wrong DevTools target;
- a missing binder, bad Mojo, or unexpected crash appears;
- File System Access/default-deny permission behavior regresses;
- the implementation adds native/passkey product UI or persistent credential
  behavior.

## Non-Negotiable Invariants

- This experiment proves contained virtual-authenticator coverage only.
- Do not claim real WebAuthn/passkey support.
- Do not add native UI or manual verification.
- Do not add protocol messages unless a later product WebAuthn issue explicitly
  designs them.
- Do not use `ninja`; Chromium builds must use `autoninja` if Chromium is
  touched.
- Run `cargo fmt` after Rust edits and accept its output if Rust is touched.

## Result

**Result:** Pass

Experiment 11 added contained, fully automated WebAuthn virtual-authenticator
coverage to the Issue 799 harness. It did not change Chromium product behavior,
add native UI, add protocol messages, or claim real passkey/security-key
support.

Changes made:

- Added `scripts/issue-799-webauthn-virtual-authenticator.mjs`, a small DevTools
  Protocol helper that:
  - polls `/json/list` for the `webauthn-create` page target;
  - requires a `page` target whose URL contains `/probe/webauthn-create.html`;
  - sends `WebAuthn.enable`;
  - sends `WebAuthn.addVirtualAuthenticator`;
  - records target evidence and `authenticatorId`;
  - keeps the DevTools session alive long enough for the page to use the virtual
    authenticator.
- Updated the `webauthn-create` probe so it waits for synthetic activation
  before calling `navigator.credentials.create()`.
- Added WebAuthn-specific harness setup and verification.
- Added the classification:

```text
webauthn_virtual_authenticator_completed
```

The WebAuthn verifier now requires concrete `PublicKeyCredential` evidence:

```text
credential.type == "public-key"
credential.id is nonempty
credential.rawId.byteLength > 0
credential.response.attestationObject.byteLength > 0
credential.response.clientDataJSON.byteLength > 0
```

Verification:

```text
node --check scripts/issue-799-webauthn-virtual-authenticator.mjs
python3 -m py_compile scripts/test-issue-799-browser-api-audit.py
git diff --check
```

Focused WebAuthn run:

```text
logs/issue-799-browser-api-audit/20260531-030916-942921
status: completed
webauthn-create: webauthn_virtual_authenticator_completed
missing_interfaces: []
empty_interfaces: []
```

Credential evidence from the focused run:

```text
type: public-key
id: nonempty
rawIdByteLength: 32
attestationObjectByteLength: 759
clientDataJSONByteLength: 117
```

Experiment 10 regression run:

```text
logs/issue-799-browser-api-audit/20260531-030936-546652
status: completed
permissions-query: default_denied
geolocation-deny: default_denied
notification-permission: default_denied
file-system-access: file_system_access_denied
service-worker-basic: exercised
missing_interfaces: []
empty_interfaces: []
```

Full Issue 799 harness run:

```text
logs/issue-799-browser-api-audit/20260531-031020-809396
status: completed
probe_count: 24
missing_interfaces: []
empty_interfaces: []
webauthn-create: webauthn_virtual_authenticator_completed
renderer-crash-recovery: renderer_crash_recovered
```

Codex reviewed the design, identified a real design gap around credential
evidence, and approved the revised design. Codex then reviewed the
implementation and verification evidence and found no blocking findings.

## Conclusion

The remaining `blocked_needs_virtual_authenticator` classification is gone.
Issue 799's automated harness now covers WebAuthn's crash-safe virtual
authenticator path with real credential material and without manual interaction.

This still does not mean TermSurf supports real WebAuthn/passkeys as a product
feature. Native authenticator UX, credential storage, platform passkey flows,
and security-key user experience remain deferred to a separate product issue if
TermSurf decides to support them.

With Experiment 11 complete, the current Issue 799 harness has no missing
interfaces, no empty interfaces, no unexpected crashes, and no remaining
`blocked_*` classifications in the full run.
