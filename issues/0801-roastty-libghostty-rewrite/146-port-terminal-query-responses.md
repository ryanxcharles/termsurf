# Experiment 146: Port Terminal Query Responses

## Description

Port Ghostty's basic terminal query response path into Roastty.

This experiment covers the query sequences that do not mutate screen contents
but ask the terminal to write a response back to the PTY:

- ENQ (`0x05`)
- Device Attributes:
  - DA1: `CSI c`
  - DA2: `CSI > c`
  - DA3: `CSI = c`
  - DECID alias: `ESC Z`
- Device Status Reports:
  - operating status: `CSI 5 n`
  - cursor position: `CSI 6 n`
  - color scheme query: `CSI ? 996 n`
- XTVERSION: `CSI > 0 q`

Roastty already has the PTY response buffer used by DECRQM and color queries.
This experiment should reuse that buffer and keep the implementation in the same
stream action plus terminal runtime shape as prior parser/runtime work.

## Changes

1. Add terminal query value types:
   - Add `roastty/src/terminal/device_attributes.rs`.
   - Add `roastty/src/terminal/device_status.rs`.
   - Export both modules from `roastty/src/terminal/mod.rs`.
   - Port Ghostty's default device attribute encoders with Roastty naming:
     - DA1 default response: `ESC [ ? 62 ; 22 c`
     - DA2 default response: `ESC [ > 1 ; 0 ; 0 c`
     - DA3 default response: `ESC P ! | 00000000 ESC \`
   - Keep the device attribute model small but typed: primary conformance level,
     primary features, secondary device type/version/cartridge, and tertiary
     unit id.
   - Port device status request parsing types for:
     - operating status (`5`)
     - cursor position (`6`)
     - color scheme (`?996`)

2. Extend `roastty/src/terminal/stream.rs`:
   - Add `Action::Enquiry`.
   - Add `Action::DeviceAttributes { request }`.
   - Add `Action::DeviceStatus { request }`.
   - Add `Action::XtVersion`.
   - Dispatch `0x05` from ground state as `Action::Enquiry`.
   - Dispatch only parameterless `CSI c`, `CSI > c`, and `CSI = c` as
     DA1/DA2/DA3.
   - Treat `CSI 0 c`, `CSI > 0 c`, `CSI = 0 c`, and other parameter-bearing DA
     forms as malformed no-action sequences for now. Ghostty currently accepts
     those because its DA dispatch ignores params; this is an intentional
     Roastty tightening until a future experiment decides whether param-bearing
     DA aliases are useful compatibility surface.
   - Dispatch `ESC Z` as DA1.
   - Dispatch `CSI 5 n`, `CSI 6 n`, and `CSI ? 996 n` as device status requests.
   - Dispatch only `CSI > 0 q` as XTVERSION. Treat `CSI > q`, `CSI > 1 q`, and
     multi-param `CSI > 0 ; 1 q` as malformed no-action sequences for now.
     Ghostty currently dispatches XTVERSION for `CSI > q` regardless of params;
     Roastty intentionally starts narrower because the existing parser keeps `>`
     as the private marker and this experiment's concrete compatibility target
     is the common xterm-style `CSI > 0 q` query.
   - Reject malformed variants without leaking final bytes as printable text:
     extra params, unexpected private markers, unexpected intermediates,
     separator-bearing params, and unknown DSR numbers.

3. Extend `roastty/src/terminal/terminal.rs` runtime behavior:
   - ENQ is a no-op for now because Ghostty only writes an ENQ response when an
     embedder effect is installed. Roastty has no app/effects layer yet.
   - Device attributes use built-in Roastty defaults and write directly to the
     existing PTY response buffer.
   - XTVERSION writes exactly `ESC P > | libroastty ESC \`. This is the
     lower-case Roastty rename of Ghostty's default `libghostty` response.
     Responses must not contain `ghostty`, `Ghostty`, or product-display
     `Roastty`.
   - Operating status writes `ESC [ 0 n`.
   - Cursor position writes `ESC [ <row> ; <col> R`, using 1-based coordinates.
   - Cursor position respects origin mode like Ghostty:
     - when origin mode is disabled, report the absolute cursor position;
     - when origin mode is enabled, subtract the current scrolling-region top
       and left margins before converting to 1-based coordinates.
   - Color scheme query is a no-op for now because Ghostty only responds when an
     embedder color-scheme effect is installed. Roastty has no app/effects layer
     yet.

4. Add tests:
   - Value-type tests for DA1/DA2/DA3 default and custom encodings.
   - Device status request parser tests for known and unknown request numbers.
   - Stream tests proving:
     - ENQ dispatches `Action::Enquiry`;
     - `CSI c`, `CSI > c`, `CSI = c`, and `ESC Z` dispatch the right device
       attribute actions;
     - `CSI 5 n`, `CSI 6 n`, and `CSI ? 996 n` dispatch the right device status
       actions;
     - `CSI > 0 q` dispatches XTVERSION;
     - malformed query variants do not dispatch actions and do not leak bytes.
   - Stream rejection tests must include DA parameter forms (`CSI 0 c`,
     `CSI > 0 c`, `CSI = 0 c`) and non-target XTVERSION forms (`CSI > q`,
     `CSI > 1 q`, `CSI > 0 ; 1 q`).
   - Terminal tests proving:
     - ENQ does not mutate display or response state yet;
     - DA1/DA2/DA3 and DECID write the expected PTY responses;
     - XTVERSION writes the exact renamed `ESC P > | libroastty ESC \` response
       and contains no `ghostty`, `Ghostty`, or `Roastty` bytes;
     - operating status writes `ESC [ 0 n`;
     - cursor position writes absolute 1-based coordinates;
     - cursor position under origin mode reports coordinates relative to the
       current scrolling-region top/left margins;
     - the origin-mode cursor-position test sets or reaches an absolute cursor
       inside a nonzero top/left region, then verifies the response equals
       `(cursor - margin + 1)`. The test must use both a nonzero top margin and
       a nonzero left margin so it cannot pass by checking only row behavior.
     - color scheme query is inert until an app/effects layer exists.

5. Keep out of scope:
   - Do not add an app/effects callback layer in this experiment.
   - Do not implement configurable/custom device attributes in `Terminal` yet.
     The value types may support custom encoding for tests and future app
     integration, but runtime should use defaults.
   - Do not implement app-controlled ENQ, XTVERSION, or color-scheme effects in
     this experiment.
   - Do not implement DCS, APC, Kitty graphics, or tmux control mode here.

## Verification

1. Run formatting:

   ```bash
   cargo fmt
   ```

2. Run focused tests:

   ```bash
   cargo test -p roastty device_attributes
   cargo test -p roastty device_status
   cargo test -p roastty query_response
   ```

3. Run the full Roastty test suite:

   ```bash
   cargo test -p roastty
   ```

## Design Review

Codex reviewed the initial design and agreed the scope was right, but did not
approve until these design details were pinned down:

- DA accepts only parameterless query forms in this experiment;
  parameter-bearing DA forms are intentionally rejected even though Ghostty
  currently ignores DA params.
- XTVERSION accepts only `CSI > 0 q` in this experiment; `CSI > q` and other
  parameter forms are intentionally rejected even though Ghostty currently
  dispatches more broadly.
- The origin-mode CPR test must prove both top-margin and left-margin
  subtraction from an absolute cursor position.
- The XTVERSION response must be exactly lower-case `libroastty` and must not
  leak any Ghostty spelling.
- Verification now uses `cargo fmt` and focused test substrings that the
  implementation must create.

Codex approved the revised design after those updates. No remaining required
design fixes.
