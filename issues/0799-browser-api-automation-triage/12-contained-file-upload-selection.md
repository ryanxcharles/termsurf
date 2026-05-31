# Experiment 12: Add Contained File-Upload Selection

## Description

Experiment 1 kept `<input type=file>` in Issue 799 only if the first
implementation used a contained automation path. The current TermSurf Chromium
path does not provide one:

- `Shell::RunFileChooser()` delegates to
  `ShellPlatformDelegate::RunFileChooser()`;
- TermSurf's `TsShellPlatformDelegate` only overrides JavaScript dialogs;
- the base non-iOS `ShellPlatformDelegate::RunFileChooser()` cancels the
  selection;
- a native `NSOpenPanel`-style product picker is explicitly deferred from this
  issue because it is native UI and not fully automatable.

This experiment adds the smallest contained path that can prove Chromium's
normal file upload pipeline works inside TermSurf: an explicit command-line
switch that auto-selects a known local fixture file for upload file choosers.

This is not the final user-facing upload UX. It is an automation and plumbing
gate. If it passes, Issue 799 can claim that file-upload browser plumbing is
safe and testable, while the eventual interactive chooser remains a separate
product/UI issue.

## Changes

1. Create a new Chromium branch from the current Issue 799 branch:
   `148.0.7778.97-issue-799-exp12`.

   Add it to `chromium/README.md` and archive it under
   `chromium/patches/issue-799/` after the Chromium commit.

2. Add a TermSurf-owned file chooser helper in `libtermsurf_chromium`.

   Preferred shape:
   - add `ts_file_select_helper.{h,cc}`;
   - expose:

     ```cpp
     bool MaybeRunTsFileChooser(
         content::RenderFrameHost* render_frame_host,
         scoped_refptr<content::FileSelectListener> listener,
         const blink::mojom::FileChooserParams& params);
     ```

   - read a new switch:

     ```text
     --termsurf-file-upload-auto-select=/absolute/path/to/file
     ```

   - if the switch is absent, return `false` so existing behavior is preserved;
   - if the switch is present and invalid or unsafe, log a
     `[termsurf-file-upload]` warning, call `listener->FileSelectionCanceled()`,
     and return `true`;
   - if the switch is present, the path is valid, and the chooser request is an
     ordinary read-only upload request, call `listener->FileSelected(...)` with
     a `blink::mojom::FileChooserFileInfo::NewNativeFile(...)` entry for that
     exact path, then return `true`.

   Path validation is load-bearing. The helper must reject:
   - relative paths;
   - empty paths;
   - nonexistent paths;
   - directories;
   - symlinks, unless the implementation explicitly resolves them first and
     records both original and resolved path in the log;
   - non-regular files.

   The selected path is an intentional file-disclosure mechanism for automation,
   so the helper should log the selected basename and hash/size evidence if easy
   to compute, but it must not silently accept ambiguous paths.

3. Support only ordinary file uploads in this experiment.

   The helper should handle:
   - `blink::mojom::FileChooserParams::Mode::kOpen`;
   - `blink::mojom::FileChooserParams::Mode::kOpenMultiple`, by returning the
     same single selected fixture file.

   It should only handle those modes when all of these fields are safe for a
   normal `<input type=file>` upload:
   - `params.need_local_path == true`;
   - `params.use_media_capture == false`;
   - `params.open_writable == false`.

   It should cancel and log unsupported or unsafe combinations, especially:
   - `kUploadFolder`;
   - `kOpenDirectory`;
   - `kSave`.

   Directory upload, save mode, native panels, drag-and-drop file drops, and a
   real protocol-mediated user picker are out of scope for Experiment 12.

4. Wire the helper through TermSurf's existing platform delegate.

   Modify `TsShellPlatformDelegate` in `ts_javascript_dialog_manager.{h,cc}` or
   split the delegate into a clearer file if that is the smallest readability
   improvement.

   `TsShellPlatformDelegate::RunFileChooser()` should:
   - call `MaybeRunTsFileChooser(...)`;
   - return immediately when the helper handled the request;
   - otherwise call the base `ShellPlatformDelegate::RunFileChooser(...)`.

   Do not modify Chromium's global `ShellFileSelectHelper`, do not add native
   dialog UI, and do not touch Wezboard or webtui for this experiment.

5. Extend `scripts/test-issue-799-browser-api-audit.py`.

   Add a new probe:

   ```text
   file-upload-input
   ```

   The local fixture should:
   - create a deterministic text file under the probe's run directory;
   - launch Roamium for this probe with
     `--termsurf-file-upload-auto-select=<fixture>`;
   - serve a page with `<input type="file">` and a submit/upload action;
   - synthesize the click on the file input only after the page reports `ready`,
     matching the activation discipline used by File System Access and WebAuthn;
   - have the page report `input.files.length`, `input.files[0].name`,
     `input.files[0].size`, and `input.files[0].type` after selection;
   - submit the selected file to a local `/upload` endpoint;
   - submit via the actual file input, either with a real
     `<form enctype="multipart/form-data">` submission or `new FormData(form)`
     where the form owns the file input;
   - have the local server parse the multipart upload and record:
     - request `Content-Type`;
     - multipart boundary presence;
     - file part `Content-Disposition`;
     - part field name;
     - part filename;
     - part byte length;
     - part SHA-256;
     - content preview in an artifact.

   The probe passes only if:
   - activation was sent and observed;
   - the page observed a non-empty `input.files` list;
   - the observed `input.files[0]` name and size match the fixture;
   - the server request is `multipart/form-data` with a boundary;
   - the server saw a file part whose `Content-Disposition` contains the
     expected field name and fixture basename;
   - the local server received exactly the fixture bytes;
   - filename and SHA-256 match the fixture;
   - there are no missing binder, bad Mojo, renderer crash, browser crash, or
     process-exit classifications.

6. Add one negative/default check.

   The experiment must also prove that the auto-select path is opt-in. Either:
   - add a second probe that omits the switch and expects a clean cancellation;
     or
   - run the same probe once without the switch in focused verification.

   A pass requires no native picker, no hang, no crash, and a clear
   `file_upload_cancelled` / `file_upload_no_auto_select` classification. This
   prevents the hidden automation switch from becoming an always-on file
   disclosure path.

7. Update the harness classification map.

   Add precise classifications rather than reusing vague `exercised`:
   - `file_upload_completed`;
   - `file_upload_cancelled`;
   - `file_upload_failed`.

   The full Issue 799 run should include the successful upload probe. The
   cancellation/default check may be focused-only if adding it to the full run
   would make the all-green summary ambiguous.

8. Format, build, and archive.
   - run Chromium's `clang-format` on modified C++ files;
   - build `libtermsurf_chromium` with `autoninja`;
   - build Roamium if needed;
   - run `python3 -m py_compile scripts/test-issue-799-browser-api-audit.py`;
   - run `git diff --check`;
   - regenerate `chromium/patches/issue-799/`;
   - run Prettier on edited Markdown files.

## Verification

1. Run the focused file-upload probe with the auto-select switch path enabled by
   the harness:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py \
     --probe file-upload-input \
     --seconds 10
   ```

   Expected:
   - probe classification is `file_upload_completed`;
   - report/artifact shows activation was sent and observed;
   - page saw exactly one selected file;
   - page-reported filename and size match the fixture;
   - upload server saw the fixture filename;
   - upload server recorded `multipart/form-data`, a boundary, file-part
     `Content-Disposition`, field name, byte count, and SHA-256 matching the
     fixture;
   - Roamium stderr contains `[termsurf-file-upload] selected ...`;
   - Roamium stderr shows the chooser was safe: `mode=kOpen` or
     `mode=kOpenMultiple`, `need_local_path=1`, `use_media_capture=0`, and
     `open_writable=0`;
   - no bad Mojo, missing binder, renderer crash, or process exit.

2. Run the focused no-auto-select negative check.

   The exact command depends on the harness implementation, but the result must
   omit `--termsurf-file-upload-auto-select`.

   Expected:
   - classification is `file_upload_cancelled`;
   - no file bytes are uploaded;
   - no native dialog is opened;
   - the probe exits cleanly without hanging or crashing;
   - Roamium stderr does not log a selected fixture path.

3. Run the full Issue 799 harness:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py --seconds 8
   ```

   Expected:
   - `file-upload-input` is included and classified as `file_upload_completed`;
   - existing Experiment 10 and 11 classifications remain stable:
     `file-system-access` is `file_system_access_denied` and `webauthn-create`
     is `webauthn_virtual_authenticator_completed`;
   - `missing_interfaces` and `empty_interfaces` are empty;
   - no unexpected `blocked_*`, crash, or process-exit classifications appear.

4. Run Codex completion review on the implementation, logs, and result before
   committing the completed experiment record.

## Pass Criteria

- TermSurf can auto-select an explicit local fixture file for a normal
  `<input type=file>` chooser through Chromium's real file chooser listener.
- The helper rejects unsafe chooser variants such as writable File System Access
  requests, media-capture requests, directory uploads, save mode, and invalid
  paths.
- The local upload server receives the exact fixture bytes and records matching
  multipart filename/hash evidence.
- The hidden auto-select path is strictly opt-in and absent by default.
- Native file picker UI remains deferred and is not introduced by this
  experiment.
- The full Issue 799 harness remains green.

## Failure Criteria

- The implementation opens a native file picker or requires manual selection.
- The upload is faked in JavaScript without exercising Chromium's file chooser
  listener.
- The auto-select path is active without an explicit switch.
- The page observes a selected file but the server does not receive matching
  bytes.
- Any previous Issue 799 probe regresses.
- The implementation broadens into drag-and-drop, directory upload, or a full
  user-facing upload UX.
