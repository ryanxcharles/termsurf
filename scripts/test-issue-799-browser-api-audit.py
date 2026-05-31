#!/usr/bin/env python3
"""Run automated browser API no-crash probes against Roamium.

The harness launches Roamium behind a minimal fake TermSurf GUI socket, serves
local probe pages, records JavaScript reports, and scans Chromium/Roamium logs
for missing Mojo binder and renderer-crash signatures.
"""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import html
import http.server
import json
import os
import pathlib
import re
import socket
import socketserver
import struct
import subprocess
import sys
import tempfile
import threading
import time
from dataclasses import dataclass
from typing import Any
from urllib.parse import parse_qs, urlparse


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_ROAMIUM = ROOT / "chromium/src/out/Default/roamium"
DEFAULT_LOG_ROOT = ROOT / "logs/issue-799-browser-api-audit"
HTTP_AUTH_USERNAME = "termsurf"
HTTP_AUTH_PASSWORD = "correct horse battery staple"
HTTP_AUTH_REALM = "TermSurf Issue 799"
HTTP_AUTH_NONCE = "issue799"

BAD_MOJO_PATTERNS = [
    "Terminating render process for bad Mojo message",
    "No binder found for interface",
    "Received bad user message",
]
CRASH_PATTERNS = [
    "RenderProcessGone",
    "bad_message",
    "CHECK failed",
    "Received signal",
]
MISSING_INTERFACE_RE = re.compile(r"No binder found for interface ([^\s]+)")
EMPTY_BINDER_RE = re.compile(r"Empty binder for interface ([^\s]+)")

ATTACHMENT_DOWNLOAD_BYTES = b"TermSurf generic attachment download fixture.\n"
BLOB_DOWNLOAD_TEXT = "TermSurf generic blob download fixture.\n"
BLOB_DOWNLOAD_BYTES = BLOB_DOWNLOAD_TEXT.encode("utf-8")
EXPECTED_DOWNLOADS = {
    "download-attachment": (
        "termsurf-download.txt",
        ATTACHMENT_DOWNLOAD_BYTES,
    ),
    "download-blob": (
        "termsurf-blob-download.txt",
        BLOB_DOWNLOAD_BYTES,
    ),
}
TERMSURF_META_MODIFIER = 1 << 3
VK_OEM_PLUS = 187
VK_OEM_MINUS = 189
VK_0 = 48
VK_A = 65
DEVTOOLS_RE = re.compile(r"DevTools listening on ws://127\.0\.0\.1:(\d+)/")


@dataclass(frozen=True)
class Probe:
    name: str
    feature: str
    expected_surface: str
    reference_evidence: str
    termsurf_evidence: str
    requires_user_activation: bool
    script: str


PROBES: list[Probe] = [
    Probe(
        name="badge",
        feature="Badging API",
        expected_surface="blink.mojom.BadgeService frame binder",
        reference_evidence="Headless has StubBadgeService; Issue 655 copied that pattern.",
        termsurf_evidence="TsBrowserClient registers StubBadgeService.",
        requires_user_activation=False,
        script="""
await navigator.setAppBadge?.(1);
await navigator.clearAppBadge?.();
return {status: navigator.setAppBadge ? 'resolved' : 'unsupported'};
""",
    ),
    Probe(
        name="permissions-query",
        feature="Permissions API",
        expected_surface="PermissionController / permission manager delegate",
        reference_evidence="Content shell has ShellPermissionManager; Chrome has full permission stack.",
        termsurf_evidence="No broad TermSurf permission manager found outside PDF-specific paths.",
        requires_user_activation=False,
        script="""
if (!navigator.permissions?.query) return {status: 'unsupported'};
const names = ['geolocation', 'notifications', 'camera', 'microphone'];
const results = [];
for (const name of names) {
  try {
    const result = await navigator.permissions.query({name});
    results.push({name, status: 'resolved', state: result.state});
  } catch (error) {
    results.push({name, status: 'rejected', error: String(error), errorName: error?.name || null});
  }
}
return {status: 'resolved', results};
""",
    ),
    Probe(
        name="notification-permission",
        feature="Notifications",
        expected_surface="Notification permission service / browser notification delegate",
        reference_evidence="Chrome wires notifications and permissions; headless provides permission behavior.",
        termsurf_evidence="No generic TermSurf notification service or OS notification path.",
        requires_user_activation=False,
        script="""
if (!('Notification' in window)) return {status: 'unsupported'};
const result = await Notification.requestPermission();
return {status: 'resolved', permission: result};
""",
    ),
    Probe(
        name="geolocation-deny",
        feature="Geolocation",
        expected_surface="Geolocation provider and permission delegate",
        reference_evidence="ContentBrowserClient exposes geolocation hooks; headless has platform geolocation handling.",
        termsurf_evidence="No TermSurf geolocation UX or fake provider path.",
        requires_user_activation=False,
        script="""
if (!navigator.geolocation?.getCurrentPosition) return {status: 'unsupported'};
return await new Promise((resolve) => {
  navigator.geolocation.getCurrentPosition(
    (position) => resolve({status: 'resolved', coords: !!position?.coords}),
    (error) => resolve({status: 'rejected', code: error.code, message: error.message}),
    {timeout: 750, maximumAge: 0}
  );
});
""",
    ),
    Probe(
        name="credential-get-empty",
        feature="Credential Management",
        expected_surface="Credential manager / WebAuthn delegate paths",
        reference_evidence="Chrome has credential delegates; WebAuthn can be tested with virtual authenticators.",
        termsurf_evidence="No TermSurf credential or WebAuthn delegate found.",
        requires_user_activation=False,
        script="""
if (!navigator.credentials?.get) return {status: 'unsupported'};
try {
  const result = await navigator.credentials.get({password: true, mediation: 'silent'});
  return {status: 'resolved', value: result ? result.type : null};
} catch (error) {
  return {status: 'rejected', error: String(error), errorName: error?.name || null};
}
""",
    ),
    Probe(
        name="webauthn-create",
        feature="WebAuthn",
        expected_surface="WebAuthenticationDelegate and authenticator request service",
        reference_evidence="Chrome wires WebAuthn; DevTools has virtual authenticator support.",
        termsurf_evidence="No TermSurf WebAuthn delegate or virtual authenticator harness yet.",
        requires_user_activation=True,
        script="""
if (!navigator.credentials?.create || !window.PublicKeyCredential) return {status: 'unsupported'};
const challenge = new Uint8Array(16);
const userId = new Uint8Array(16);
const button = document.createElement('button');
button.textContent = 'create WebAuthn credential';
button.style.position = 'absolute';
button.style.left = '8px';
button.style.top = '8px';
button.style.width = '220px';
button.style.height = '48px';
document.body.appendChild(button);
let activationObserved = false;
button.onpointerdown = () => sendReport({status: 'pointerdown'});
button.onmousedown = () => sendReport({status: 'mousedown'});
function credentialEvidence(credential) {
  return {
    type: credential?.type || null,
    id: credential?.id || null,
    rawIdByteLength: credential?.rawId?.byteLength || 0,
    attestationObjectByteLength:
      credential?.response?.attestationObject?.byteLength || 0,
    clientDataJSONByteLength:
      credential?.response?.clientDataJSON?.byteLength || 0
  };
}
return await new Promise((resolve) => {
  button.onclick = async () => {
    activationObserved = true;
    sendReport({status: 'activated'});
    try {
      const result = await Promise.race([
        navigator.credentials.create({
          publicKey: {
            challenge,
            rp: {name: 'TermSurf Test'},
            user: {
              id: userId,
              name: 'test@example.test',
              displayName: 'Test User'
            },
            pubKeyCredParams: [{type: 'public-key', alg: -7}],
            timeout: 2000,
            attestation: 'direct',
            authenticatorSelection: {
              residentKey: 'preferred',
              userVerification: 'preferred'
            }
          }
        }),
        new Promise((resolveTimeout) => setTimeout(
          () => resolveTimeout(null),
          2500
        ))
      ]);
      if (!result) {
        resolve({
          status: 'blocked_needs_virtual_authenticator',
          activationObserved
        });
        return;
      }
      resolve({
        status: 'webauthn_virtual_authenticator_created',
        activationObserved,
        credential: credentialEvidence(result)
      });
    } catch (error) {
      const blocked = error?.name === 'NotAllowedError' &&
        /user activation|gesture/i.test(String(error?.message || error));
      resolve({
        status: blocked ? 'blocked_user_activation' : 'rejected',
        activationObserved,
        error: String(error),
        errorName: error?.name || null,
        message: error?.message || null
      });
    }
  };
  sendReport({status: 'ready'});
});
""",
    ),
    Probe(
        name="file-system-access",
        feature="File System Access",
        expected_surface="File-system access permission context and native picker delegate",
        reference_evidence="Chrome has file-system access permission/picker plumbing.",
        termsurf_evidence="No TermSurf file-system access picker or permission UX.",
        requires_user_activation=True,
        script="""
if (!window.showOpenFilePicker) return {status: 'unsupported'};
const button = document.createElement('button');
button.textContent = 'open file picker';
button.style.position = 'absolute';
button.style.left = '8px';
button.style.top = '8px';
button.style.width = '180px';
button.style.height = '48px';
document.body.appendChild(button);
let activationObserved = false;
let pickerCallStartedAfterActivation = false;
button.onpointerdown = () => sendReport({status: 'pointerdown'});
button.onmousedown = () => sendReport({status: 'mousedown'});
return await new Promise((resolve) => {
  button.onclick = async () => {
    activationObserved = true;
    sendReport({status: 'activated'});
    pickerCallStartedAfterActivation = true;
    sendReport({status: 'picker_call_started_after_activation'});
    try {
      await window.showOpenFilePicker({multiple: false});
      resolve({
        status: 'resolved',
        activationObserved,
        pickerCallStartedAfterActivation
      });
    } catch (error) {
      resolve({
        status: 'file_system_access_denied',
        activationObserved,
        pickerCallStartedAfterActivation,
        error: String(error),
        errorName: error?.name || null,
        message: error?.message || null
      });
    }
  };
  sendReport({status: 'ready'});
});
""",
    ),
    Probe(
        name="payment-request",
        feature="Payment Request",
        expected_surface="Payment app/service delegate and permission/product UI",
        reference_evidence="Chrome has payment service stack; content embedders often omit full feature.",
        termsurf_evidence="No TermSurf payment request implementation.",
        requires_user_activation=False,
        script="""
if (!window.PaymentRequest) return {status: 'unsupported'};
try {
  const makeRequest = () => new PaymentRequest(
    [{supportedMethods: 'basic-card'}],
    {total: {label: 'Total', amount: {currency: 'USD', value: '1.00'}}}
  );
  const request = makeRequest();
  const canMakePayment = await request.canMakePayment();
  let hasEnrolledInstrument = null;
  if (typeof request.hasEnrolledInstrument === 'function') {
    hasEnrolledInstrument = await request.hasEnrolledInstrument();
  }
  let show = null;
  try {
    await makeRequest().show();
    show = {status: 'resolved'};
  } catch (error) {
    show = {
      status: 'rejected',
      error: String(error),
      errorName: error?.name || null
    };
  }
  return {status: 'resolved', canMakePayment, hasEnrolledInstrument, show};
} catch (error) {
  return {status: 'rejected', error: String(error), errorName: error?.name || null};
}
""",
    ),
    Probe(
        name="service-worker-basic",
        feature="Service worker browser services",
        expected_surface="Service-worker binder maps and storage/registration delegates",
        reference_evidence="Chrome/content shell support service-worker registration paths.",
        termsurf_evidence="No systematic TermSurf service-worker browser-service audit yet.",
        requires_user_activation=False,
        script="""
if (!navigator.serviceWorker?.register) return {status: 'unsupported'};
try {
  return await Promise.race([
    (async () => {
      const registration = await navigator.serviceWorker.register(
        '/probe/service-worker-basic-worker.js',
        {scope: '/probe/'}
      );
      const worker = registration.installing || registration.waiting || registration.active;
      if (worker && worker.state !== 'activated') {
        await new Promise((resolve) => {
          worker.addEventListener('statechange', () => {
            if (worker.state === 'activated') resolve();
          });
        });
      }
      await registration.unregister();
      return {status: 'resolved'};
    })(),
    new Promise((resolve) => setTimeout(() => resolve({status: 'probe_timeout'}), 2500))
  ]);
} catch (error) {
  return {status: 'rejected', error: String(error), errorName: error?.name || null};
}
""",
    ),
    Probe(
        name="download-attachment",
        feature="Generic attachment downloads",
        expected_surface="Content download manager delegate and deterministic target path",
        reference_evidence="Content Shell has ShellDownloadManagerDelegate; Chrome has full download UI.",
        termsurf_evidence="TermSurf needs contained no-prompt generic download target selection.",
        requires_user_activation=False,
        script="""
const link = document.createElement('a');
link.href = '/download/attachment.txt';
link.textContent = 'download attachment';
document.body.appendChild(link);
setTimeout(() => link.click(), 250);
return {status: 'download_triggered', expectedFile: 'termsurf-download.txt'};
""",
    ),
    Probe(
        name="download-blob",
        feature="Generic blob downloads",
        expected_surface="Content download manager delegate and deterministic target path",
        reference_evidence="Chrome downloads Blob URLs through the generic download stack.",
        termsurf_evidence="TermSurf needs contained no-prompt generic download target selection.",
        requires_user_activation=False,
        script=f"""
const blob = new Blob([{json.dumps(BLOB_DOWNLOAD_TEXT)}], {{type: 'text/plain'}});
const link = document.createElement('a');
link.href = URL.createObjectURL(blob);
link.download = 'termsurf-blob-download.txt';
link.textContent = 'download blob';
document.body.appendChild(link);
setTimeout(() => link.click(), 250);
return {{status: 'download_triggered', expectedFile: 'termsurf-blob-download.txt'}};
""",
    ),
    Probe(
        name="page-zoom-shortcuts",
        feature="Page zoom",
        expected_surface="Chromium page zoom controller and TermSurf Meta-key command routing",
        reference_evidence="Chrome routes Cmd+=/-/0 to components/zoom PageZoom.",
        termsurf_evidence="Experiment 6 adds Chromium-side page zoom shortcut handling.",
        requires_user_activation=False,
        script="""
const keyEvents = [];
function keySnapshot(event) {
  return {
    type: event.type,
    key: event.key,
    code: event.code,
    keyCode: event.keyCode,
    metaKey: event.metaKey
  };
}
document.addEventListener('keydown', (event) => {
  const snapshot = keySnapshot(event);
  keyEvents.push(snapshot);
  sendReport({status: 'key_event', event: snapshot});
});
document.addEventListener('keyup', (event) => {
  const snapshot = keySnapshot(event);
  keyEvents.push(snapshot);
  sendReport({status: 'key_event', event: snapshot});
});
const marker = document.createElement('div');
marker.textContent = 'TermSurf page zoom marker';
marker.style.cssText = 'width: 240px; height: 40px; padding: 10px; font-size: 20px;';
document.body.appendChild(marker);
function collectMetrics(label) {
  const rect = marker.getBoundingClientRect();
  return {
    status: 'page_zoom_metrics',
    label,
    devicePixelRatio: window.devicePixelRatio,
    innerWidth: window.innerWidth,
    clientWidth: document.documentElement.clientWidth,
    visualViewportWidth: window.visualViewport ? window.visualViewport.width : null,
    boxWidth: rect.width,
    keyEvents: keyEvents.slice()
  };
}
let metricTimer = null;
function scheduleMetrics(label) {
  if (metricTimer !== null) clearTimeout(metricTimer);
  metricTimer = setTimeout(() => sendReport(collectMetrics(label)), 120);
}
window.addEventListener('resize', () => scheduleMetrics('window-resize'));
window.visualViewport?.addEventListener('resize', () => scheduleMetrics('visual-viewport-resize'));
setTimeout(() => sendReport(collectMetrics('baseline')), 150);
let metricPollCount = 0;
const metricPoll = setInterval(() => {
  metricPollCount += 1;
  sendReport(collectMetrics('poll-' + metricPollCount));
  if (metricPollCount >= 30) clearInterval(metricPoll);
}, 250);
return {status: 'ready'};
""",
    ),
    Probe(
        name="console-capture-basic",
        feature="Console capture",
        expected_surface="WebContentsObserver console event routed through TermSurf protocol",
        reference_evidence="Chromium exposes WebContentsObserver::OnDidAddMessageToConsole.",
        termsurf_evidence="Experiment 7 adds one-way ConsoleMessage protocol events.",
        requires_user_activation=False,
        script="""
const nonce = 'issue799';
console.log('ts-console-top-log-' + nonce);
console.info('ts-console-top-info-' + nonce);
console.warn('ts-console-top-warn-' + nonce);
console.error('ts-console-top-error-' + nonce);
const iframe = document.createElement('iframe');
iframe.src = '/probe/console-capture-basic-frame.html?nonce=' + encodeURIComponent(nonce);
document.body.appendChild(iframe);
setTimeout(() => {
  throw new Error('ts-console-throw-' + nonce);
}, 100);
await new Promise((resolve) => setTimeout(resolve, 500));
return {status: 'console_emitted', nonce};
""",
    ),
    Probe(
        name="http-basic-auth-success",
        feature="HTTP Basic Auth",
        expected_surface="TermSurf HTTP auth request/reply protocol",
        reference_evidence="Chromium embedders implement ContentBrowserClient::CreateLoginDelegate.",
        termsurf_evidence="Experiment 8 adds protocol-mediated HTTP Basic Auth.",
        requires_user_activation=False,
        script="""
return {status: 'unused'};
""",
    ),
    Probe(
        name="http-basic-auth-cancel",
        feature="HTTP Basic Auth",
        expected_surface="TermSurf HTTP auth request/reply protocol",
        reference_evidence="Chromium embedders implement ContentBrowserClient::CreateLoginDelegate.",
        termsurf_evidence="Experiment 8 adds protocol-mediated HTTP Basic Auth.",
        requires_user_activation=False,
        script="""
return {status: 'unused'};
""",
    ),
    Probe(
        name="renderer-crash-recovery",
        feature="Renderer crash recovery",
        expected_surface="WebContentsObserver renderer crash event routed through TermSurf protocol",
        reference_evidence="Chromium exposes PrimaryMainFrameRenderProcessGone and chrome://crash/ test URL.",
        termsurf_evidence="Experiment 9 adds protocol-visible RendererCrashed recovery UX.",
        requires_user_activation=False,
        script="""
return {status: 'ready'};
""",
    ),
    Probe(
        name="javascript-alert",
        feature="JavaScript dialogs",
        expected_surface="TermSurf JavaScript dialog request/reply protocol",
        reference_evidence="Content Shell opens native dialogs; TermSurf must route through protocol.",
        termsurf_evidence="Experiment 5 adds protocol-mediated dialogs.",
        requires_user_activation=False,
        script="""
alert('alpha');
return {status: 'resolved', value: 'resumed'};
""",
    ),
    Probe(
        name="javascript-confirm-accept",
        feature="JavaScript dialogs",
        expected_surface="TermSurf JavaScript dialog request/reply protocol",
        reference_evidence="Content Shell opens native dialogs; TermSurf must route through protocol.",
        termsurf_evidence="Experiment 5 adds protocol-mediated dialogs.",
        requires_user_activation=False,
        script="""
const value = confirm('beta');
return {status: 'resolved', value};
""",
    ),
    Probe(
        name="javascript-confirm-cancel",
        feature="JavaScript dialogs",
        expected_surface="TermSurf JavaScript dialog request/reply protocol",
        reference_evidence="Content Shell opens native dialogs; TermSurf must route through protocol.",
        termsurf_evidence="Experiment 5 adds protocol-mediated dialogs.",
        requires_user_activation=False,
        script="""
const value = confirm('gamma');
return {status: 'resolved', value};
""",
    ),
    Probe(
        name="javascript-prompt",
        feature="JavaScript dialogs",
        expected_surface="TermSurf JavaScript dialog request/reply protocol",
        reference_evidence="Content Shell opens native dialogs; TermSurf must route through protocol.",
        termsurf_evidence="Experiment 5 adds protocol-mediated dialogs.",
        requires_user_activation=False,
        script="""
const value = prompt('delta', 'default');
return {status: 'resolved', value};
""",
    ),
    Probe(
        name="javascript-prompt-cancel",
        feature="JavaScript dialogs",
        expected_surface="TermSurf JavaScript dialog request/reply protocol",
        reference_evidence="Content Shell opens native dialogs; TermSurf must route through protocol.",
        termsurf_evidence="Experiment 5 adds protocol-mediated dialogs.",
        requires_user_activation=False,
        script="""
const value = prompt('epsilon', 'default');
return {status: 'resolved', value};
""",
    ),
    Probe(
        name="javascript-initial-load-alert",
        feature="JavaScript dialogs",
        expected_surface="TermSurf JavaScript dialog request/reply protocol",
        reference_evidence="Content Shell opens native dialogs; TermSurf must route through protocol.",
        termsurf_evidence="Experiment 5 adds protocol-mediated dialogs.",
        requires_user_activation=False,
        script="""
alert('load');
return {status: 'resolved', value: 'initial-load-resumed'};
""",
    ),
    Probe(
        name="javascript-beforeunload-proceed",
        feature="JavaScript dialogs",
        expected_surface="TermSurf beforeunload dialog request/reply protocol",
        reference_evidence="Chromium requires sticky user activation for blocking beforeunload dialogs.",
        termsurf_evidence="Experiment 5 routes beforeunload through TermSurf dialogs.",
        requires_user_activation=True,
        script="""
const button = document.createElement('button');
button.textContent = 'activate';
button.style.position = 'absolute';
button.style.left = '8px';
button.style.top = '8px';
button.style.width = '120px';
button.style.height = '40px';
button.onpointerdown = () => sendReport({status: 'pointerdown'});
button.onmousedown = () => sendReport({status: 'mousedown'});
button.onclick = () => sendReport({status: 'activated'});
document.body.appendChild(button);
document.body.tabIndex = 0;
document.body.focus();
document.addEventListener('keydown', () => sendReport({
  status: 'activated',
  activation: 'keyboard'
}), {once: true});
window.addEventListener('beforeunload', (event) => {
  event.preventDefault();
  event.returnValue = '';
});
return {status: 'ready'};
""",
    ),
    Probe(
        name="javascript-beforeunload-stay",
        feature="JavaScript dialogs",
        expected_surface="TermSurf beforeunload dialog request/reply protocol",
        reference_evidence="Chromium requires sticky user activation for blocking beforeunload dialogs.",
        termsurf_evidence="Experiment 5 routes beforeunload through TermSurf dialogs.",
        requires_user_activation=True,
        script="""
const button = document.createElement('button');
button.textContent = 'activate';
button.style.position = 'absolute';
button.style.left = '8px';
button.style.top = '8px';
button.style.width = '120px';
button.style.height = '40px';
button.onpointerdown = () => sendReport({status: 'pointerdown'});
button.onmousedown = () => sendReport({status: 'mousedown'});
button.onclick = () => sendReport({status: 'activated'});
document.body.appendChild(button);
document.body.tabIndex = 0;
document.body.focus();
document.addEventListener('keydown', () => sendReport({
  status: 'activated',
  activation: 'keyboard'
}), {once: true});
window.addEventListener('beforeunload', (event) => {
  event.preventDefault();
  event.returnValue = '';
  setTimeout(() => sendReport({status: 'stayed'}), 500);
});
return {status: 'ready'};
""",
    ),
]


def varint(value: int) -> bytes:
    out = bytearray()
    while value >= 0x80:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value)
    return bytes(out)


def read_varint(buf: bytes, index: int) -> tuple[int, int]:
    shift = 0
    value = 0
    while index < len(buf):
        byte = buf[index]
        index += 1
        value |= (byte & 0x7F) << shift
        if not byte & 0x80:
            return value, index
        shift += 7
    return 0, index


def field(number: int, wire_type: int) -> bytes:
    return varint((number << 3) | wire_type)


def string_field(number: int, value: str) -> bytes:
    data = value.encode("utf-8")
    return field(number, 2) + varint(len(data)) + data


def varint_field(number: int, value: int) -> bytes:
    return field(number, 0) + varint(value)


def bool_field(number: int, value: bool) -> bytes:
    return field(number, 0) + varint(1 if value else 0)


def fixed_double_field(number: int, value: float) -> bytes:
    return field(number, 1) + struct.pack("<d", float(value))


def wrap(inner_field: int, payload: bytes) -> bytes:
    return field(inner_field, 2) + varint(len(payload)) + payload


def send_message(conn: socket.socket, inner_field: int, payload: bytes) -> None:
    message = wrap(inner_field, payload)
    conn.sendall(struct.pack("<I", len(message)) + message)


def inner_payload(payload: bytes) -> tuple[int, bytes]:
    key, index = read_varint(payload, 0)
    length, index = read_varint(payload, index)
    return key >> 3, payload[index : index + length]


def tab_ready_id(payload: bytes) -> int | None:
    index = 0
    while index < len(payload):
        key, index = read_varint(payload, index)
        field_number = key >> 3
        wire_type = key & 7
        if wire_type == 0:
            value, index = read_varint(payload, index)
            if field_number == 2:
                return value
        elif wire_type == 2:
            length, index = read_varint(payload, index)
            index += length
        else:
            return None
    return None


def parse_message_fields(payload: bytes) -> dict[int, Any]:
    values: dict[int, Any] = {}
    index = 0
    while index < len(payload):
        key, index = read_varint(payload, index)
        field_number = key >> 3
        wire_type = key & 7
        if wire_type == 0:
            value, index = read_varint(payload, index)
            values[field_number] = value
        elif wire_type == 2:
            length, index = read_varint(payload, index)
            data = payload[index : index + length]
            index += length
            try:
                values[field_number] = data.decode("utf-8")
            except UnicodeDecodeError:
                values[field_number] = data
        else:
            break
    return values


def javascript_dialog_reply_payload(
    tab_id: int,
    request_id: int,
    accepted: bool,
    prompt_text: str,
) -> bytes:
    return (
        varint_field(1, tab_id)
        + varint_field(2, request_id)
        + bool_field(3, accepted)
        + string_field(4, prompt_text)
    )


def http_auth_reply_payload(
    tab_id: int,
    request_id: int,
    accepted: bool,
    username: str,
    password: str,
) -> bytes:
    return (
        varint_field(1, tab_id)
        + varint_field(2, request_id)
        + bool_field(3, accepted)
        + string_field(4, username)
        + string_field(5, password)
    )


def dialog_response_for(probe_name: str) -> tuple[bool, str]:
    if probe_name in (
        "javascript-confirm-cancel",
        "javascript-prompt-cancel",
        "javascript-beforeunload-stay",
    ):
        return False, ""
    if probe_name == "javascript-prompt":
        return True, "typed value"
    return True, ""


def verify_javascript_dialog_probe(
    probe_name: str,
    report: dict[str, Any] | None,
    dialogs: list[dict[str, Any]],
    beforeunload_activation_observed: bool,
) -> dict[str, Any] | None:
    if not probe_name.startswith("javascript-"):
        return None
    expected: dict[str, Any] = {
        "javascript-alert": {"dialog_type": "alert", "message": "alpha", "value": "resumed"},
        "javascript-confirm-accept": {
            "dialog_type": "confirm",
            "message": "beta",
            "value": True,
        },
        "javascript-confirm-cancel": {
            "dialog_type": "confirm",
            "message": "gamma",
            "value": False,
        },
        "javascript-prompt": {
            "dialog_type": "prompt",
            "message": "delta",
            "value": "typed value",
        },
        "javascript-prompt-cancel": {
            "dialog_type": "prompt",
            "message": "epsilon",
            "value": None,
        },
        "javascript-initial-load-alert": {
            "dialog_type": "alert",
            "message": "load",
            "value": "initial-load-resumed",
        },
        "javascript-beforeunload-proceed": {
            "dialog_type": "beforeunload",
            "message": "Is it OK to leave this page?",
            "final_status": "destination_loaded",
        },
        "javascript-beforeunload-stay": {
            "dialog_type": "beforeunload",
            "message": "Is it OK to leave this page?",
            "final_status": "stayed",
        },
    }[probe_name]
    status = "completed"
    reasons: list[str] = []
    if len(dialogs) != 1:
        status = "failed"
        reasons.append(f"expected one dialog, got {len(dialogs)}")
    elif dialogs[0].get("dialog_type") != expected["dialog_type"]:
        status = "failed"
        reasons.append(f"wrong dialog type {dialogs[0].get('dialog_type')}")
    elif dialogs[0].get("message") != expected["message"]:
        status = "failed"
        reasons.append(f"wrong message {dialogs[0].get('message')}")
    if "final_status" in expected:
        if (
            probe_name.startswith("javascript-beforeunload-")
            and not beforeunload_activation_observed
        ):
            status = "failed"
            reasons.append("page did not report activation before navigation")
        if not report or report.get("status") != expected["final_status"]:
            status = "failed"
            reasons.append(
                f"page did not report final status {expected['final_status']}"
            )
    elif not report or report.get("status") != "resolved":
        status = "failed"
        reasons.append("page did not report resolved")
    elif report.get("value") != expected["value"]:
        status = "failed"
        reasons.append(f"wrong page value {report.get('value')!r}")
    return {"status": status, "reasons": reasons, "expected": expected}


def verify_default_deny_probe(
    probe_name: str,
    report: dict[str, Any] | None,
) -> dict[str, Any] | None:
    if probe_name == "permissions-query":
        expected_states = {
            "geolocation": "denied",
            "notifications": "denied",
            "camera": "denied",
            "microphone": "denied",
        }
        reasons: list[str] = []
        actual: dict[str, str | None] = {}
        if not report or report.get("status") != "resolved":
            reasons.append("permissions query did not resolve")
        for entry in (report or {}).get("results", []):
            name = str(entry.get("name", ""))
            if name in expected_states:
                actual[name] = entry.get("state")
        for name, expected_state in expected_states.items():
            if actual.get(name) != expected_state:
                reasons.append(
                    f"{name} expected {expected_state}, got {actual.get(name)!r}"
                )
        return {
            "status": "completed" if not reasons else "failed",
            "reasons": reasons,
            "expected_states": expected_states,
            "actual_states": actual,
        }
    if probe_name == "geolocation-deny":
        reasons = []
        if not report or report.get("status") != "rejected":
            reasons.append("geolocation did not reject")
        if (report or {}).get("code") != 1:
            reasons.append(
                "expected PERMISSION_DENIED code 1, "
                f"got {(report or {}).get('code')!r}"
            )
        return {
            "status": "completed" if not reasons else "failed",
            "reasons": reasons,
            "expected_code": 1,
        }
    if probe_name == "notification-permission":
        reasons = []
        if not report or report.get("status") != "resolved":
            reasons.append("notification permission request did not resolve")
        if (report or {}).get("permission") != "denied":
            reasons.append(
                f"expected denied, got {(report or {}).get('permission')!r}"
            )
        return {
            "status": "completed" if not reasons else "failed",
            "reasons": reasons,
            "expected_permission": "denied",
        }
    return None


def verify_file_system_access_probe(
    probe_name: str,
    report: dict[str, Any] | None,
    activation_ready_at: float | None,
    activation_sent: bool,
    activation_observed_at: float | None,
    picker_call_started_at: float | None,
) -> dict[str, Any] | None:
    if probe_name != "file-system-access":
        return None
    reasons: list[str] = []
    if activation_ready_at is None:
        reasons.append("page never reported ready")
    if not activation_sent:
        reasons.append("harness did not send activation input")
    if activation_observed_at is None:
        reasons.append("page did not observe activation")
    if picker_call_started_at is None:
        reasons.append("page did not start picker call after activation")
    if not report:
        reasons.append("page did not report final picker result")
    elif report.get("status") != "file_system_access_denied":
        reasons.append(
            f"expected file_system_access_denied, got {report.get('status')!r}"
        )
    if report and not report.get("activationObserved"):
        reasons.append("final report did not record activationObserved=true")
    if report and not report.get("pickerCallStartedAfterActivation"):
        reasons.append(
            "final report did not record pickerCallStartedAfterActivation=true"
        )
    message = str(
        (report or {}).get("message") or (report or {}).get("error") or ""
    )
    if report and "not allowed at this time" not in message:
        reasons.append("rejection did not include TermSurf file-picker policy denial")
    if report and re.search(r"user gesture|user activation", message, re.I):
        reasons.append("rejection still looks like a user-activation failure")
    return {
        "status": "completed" if not reasons else "failed",
        "reasons": reasons,
        "activation_sent": activation_sent,
        "activation_observed": activation_observed_at is not None,
        "picker_call_started_after_activation": picker_call_started_at is not None,
        "errorName": (report or {}).get("errorName"),
        "message": (report or {}).get("message"),
    }


def verify_webauthn_probe(
    probe_name: str,
    report: dict[str, Any] | None,
    activation_ready_at: float | None,
    activation_sent: bool,
    activation_observed_at: float | None,
    devtools_setup: dict[str, Any] | None,
) -> dict[str, Any] | None:
    if probe_name != "webauthn-create":
        return None
    reasons: list[str] = []
    artifact = (devtools_setup or {}).get("artifact") or {}
    target = artifact.get("target") or {}
    credential = (report or {}).get("credential") or {}
    if not devtools_setup:
        reasons.append("DevTools setup did not run")
    elif devtools_setup.get("status") != "completed":
        reasons.append(f"DevTools setup failed: {devtools_setup.get('status')}")
    if not (devtools_setup or {}).get("devtools_port"):
        reasons.append("DevTools port was not discovered")
    if not artifact.get("webAuthnEnable", {}).get("ok"):
        reasons.append("WebAuthn.enable did not succeed")
    if not artifact.get("authenticatorId"):
        reasons.append("WebAuthn.addVirtualAuthenticator returned no id")
    if target.get("type") != "page":
        reasons.append(f"DevTools target type was {target.get('type')!r}")
    if "/probe/webauthn-create.html" not in str(target.get("url", "")):
        reasons.append("DevTools target URL was not the WebAuthn probe page")
    if activation_ready_at is None:
        reasons.append("page never reported ready")
    if not activation_sent:
        reasons.append("harness did not send activation input")
    if activation_observed_at is None:
        reasons.append("page did not observe activation")
    if not report:
        reasons.append("page did not report final WebAuthn result")
    elif report.get("status") != "webauthn_virtual_authenticator_created":
        reasons.append(
            "expected webauthn_virtual_authenticator_created, "
            f"got {report.get('status')!r}"
        )
    if credential.get("type") != "public-key":
        reasons.append(f"credential type was {credential.get('type')!r}")
    if not credential.get("id"):
        reasons.append("credential id was empty")
    if int(credential.get("rawIdByteLength") or 0) <= 0:
        reasons.append("credential rawId was empty")
    if int(credential.get("attestationObjectByteLength") or 0) <= 0:
        reasons.append("credential attestationObject was empty")
    if int(credential.get("clientDataJSONByteLength") or 0) <= 0:
        reasons.append("credential clientDataJSON was empty")
    return {
        "status": "completed" if not reasons else "failed",
        "reasons": reasons,
        "devtools_port": (devtools_setup or {}).get("devtools_port"),
        "authenticatorId": artifact.get("authenticatorId"),
        "target": target,
        "activation_sent": activation_sent,
        "activation_observed": activation_observed_at is not None,
        "credential": credential,
    }


def numeric_metric(report: dict[str, Any], key: str) -> float | None:
    value = report.get(key)
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def viewport_metric(report: dict[str, Any]) -> float | None:
    for key in ("visualViewportWidth", "innerWidth", "clientWidth"):
        value = numeric_metric(report, key)
        if value is not None:
            return value
    return None


def first_metric_after(
    metrics: list[dict[str, Any]],
    sent_at: float,
) -> dict[str, Any] | None:
    for report in metrics:
        received_at = numeric_metric(report, "_received_at")
        if received_at is not None and received_at > sent_at:
            return report
    return None


def closer_to_baseline(value: float, baseline: float, previous: float) -> bool:
    return abs(value - baseline) < abs(previous - baseline)


def key_code_value(event: dict[str, Any]) -> int:
    try:
        return int(event.get("keyCode", -1))
    except (TypeError, ValueError):
        return -1


def verify_page_zoom_probe(
    probe_name: str,
    reports: list[dict[str, Any]],
    zoom_events: list[dict[str, Any]],
) -> dict[str, Any] | None:
    if probe_name != "page-zoom-shortcuts":
        return None

    status = "completed"
    reasons: list[str] = []
    metrics = [
        report for report in reports if report.get("status") == "page_zoom_metrics"
    ]
    baseline = next(
        (report for report in metrics if report.get("label") == "baseline"),
        metrics[0] if metrics else None,
    )
    by_name = {event["name"]: event for event in zoom_events}

    if not baseline:
        status = "failed"
        reasons.append("missing baseline metrics")
    for name in ("zoom-in", "zoom-out", "reset", "normal-a"):
        if name not in by_name:
            status = "failed"
            reasons.append(f"missing sent event {name}")

    zoom_in = (
        first_metric_after(metrics, by_name["zoom-in"]["sent_at"])
        if "zoom-in" in by_name
        else None
    )
    zoom_out = (
        first_metric_after(metrics, by_name["zoom-out"]["sent_at"])
        if "zoom-out" in by_name
        else None
    )
    reset = (
        first_metric_after(metrics, by_name["reset"]["sent_at"])
        if "reset" in by_name
        else None
    )

    snapshots = {
        "baseline": baseline,
        "zoom_in": zoom_in,
        "zoom_out": zoom_out,
        "reset": reset,
    }

    if baseline and zoom_in and zoom_out and reset:
        baseline_dpr = numeric_metric(baseline, "devicePixelRatio")
        zoom_in_dpr = numeric_metric(zoom_in, "devicePixelRatio")
        zoom_out_dpr = numeric_metric(zoom_out, "devicePixelRatio")
        reset_dpr = numeric_metric(reset, "devicePixelRatio")
        baseline_viewport = viewport_metric(baseline)
        zoom_in_viewport = viewport_metric(zoom_in)
        zoom_out_viewport = viewport_metric(zoom_out)
        reset_viewport = viewport_metric(reset)
        if (
            baseline_dpr is None
            or zoom_in_dpr is None
            or zoom_out_dpr is None
            or reset_dpr is None
        ):
            status = "failed"
            reasons.append("missing devicePixelRatio metric")
        elif zoom_in_dpr <= baseline_dpr + 0.01:
            status = "failed"
            reasons.append("devicePixelRatio did not increase after Cmd+=")
        elif not closer_to_baseline(zoom_out_dpr, baseline_dpr, zoom_in_dpr):
            status = "failed"
            reasons.append("devicePixelRatio did not move back after Cmd+-")
        elif abs(reset_dpr - baseline_dpr) > 0.02:
            status = "failed"
            reasons.append("devicePixelRatio did not reset to baseline")

        if (
            baseline_viewport is None
            or zoom_in_viewport is None
            or zoom_out_viewport is None
            or reset_viewport is None
        ):
            status = "failed"
            reasons.append("missing CSS viewport metric")
        elif zoom_in_viewport >= baseline_viewport - 1:
            status = "failed"
            reasons.append("CSS viewport metric did not shrink after Cmd+=")
        elif not closer_to_baseline(
            zoom_out_viewport, baseline_viewport, zoom_in_viewport
        ):
            status = "failed"
            reasons.append("CSS viewport metric did not move back after Cmd+-")
        elif abs(reset_viewport - baseline_viewport) > max(
            2.0, baseline_viewport * 0.02
        ):
            status = "failed"
            reasons.append("CSS viewport metric did not reset to baseline")
    else:
        status = "failed"
        missing = [
            name for name, report in snapshots.items() if report is None
        ]
        reasons.append(f"missing metric snapshots: {', '.join(missing)}")

    key_reports = [report for report in reports if report.get("status") == "key_event"]
    key_events = [
        report.get("event", {})
        for report in key_reports
        if isinstance(report.get("event"), dict)
    ]
    leaked_zoom_events = [
        event
        for event in key_events
        if event.get("metaKey")
        and key_code_value(event) in (VK_OEM_PLUS, VK_OEM_MINUS, VK_0)
    ]
    saw_normal_a = any(
        not event.get("metaKey") and key_code_value(event) == VK_A
        for event in key_events
    )
    if leaked_zoom_events:
        status = "failed"
        reasons.append("zoom shortcut key events reached the page")
    if not saw_normal_a:
        status = "failed"
        reasons.append("normal a key did not reach the page")

    return {
        "status": status,
        "reasons": reasons,
        "sent_events": zoom_events,
        "snapshots": snapshots,
        "key_events": key_events,
    }


def verify_console_capture_probe(
    probe_name: str,
    tab_id: int | None,
    report: dict[str, Any] | None,
    console_messages: list[dict[str, Any]],
) -> dict[str, Any] | None:
    if probe_name != "console-capture-basic":
        return None

    status = "completed"
    reasons: list[str] = []
    expected = [
        {
            "name": "top-log",
            "message": "ts-console-top-log-issue799",
            "level": "info",
            "source": "/probe/console-capture-basic.html",
        },
        {
            "name": "top-info",
            "message": "ts-console-top-info-issue799",
            "level": "info",
            "source": "/probe/console-capture-basic.html",
        },
        {
            "name": "top-warn",
            "message": "ts-console-top-warn-issue799",
            "level": "warning",
            "source": "/probe/console-capture-basic.html",
        },
        {
            "name": "top-error",
            "message": "ts-console-top-error-issue799",
            "level": "error",
            "source": "/probe/console-capture-basic.html",
        },
        {
            "name": "frame-warn",
            "message": "ts-console-frame-warn-issue799",
            "level": "warning",
            "source": "/probe/console-capture-basic-frame.html",
        },
        {
            "name": "uncaught-error",
            "message": "ts-console-throw-issue799",
            "level": "error",
            "source": "/probe/console-capture-basic.html",
        },
    ]
    matches: dict[str, dict[str, Any]] = {}
    for item in expected:
        found = next(
            (
                message
                for message in console_messages
                if item["message"] in str(message.get("message", ""))
            ),
            None,
        )
        if not found:
            status = "failed"
            reasons.append(f"missing console message {item['name']}")
            continue
        matches[item["name"]] = found
        if found.get("level") != item["level"]:
            status = "failed"
            reasons.append(
                f"wrong level for {item['name']}: {found.get('level')}"
            )
        if tab_id is not None and found.get("tab_id") != tab_id:
            status = "failed"
            reasons.append(
                f"wrong tab_id for {item['name']}: {found.get('tab_id')}"
            )
        if item["source"] not in str(found.get("source_id", "")):
            status = "failed"
            reasons.append(
                f"wrong source for {item['name']}: {found.get('source_id')}"
            )
        if int(found.get("line_no", 0) or 0) <= 0:
            status = "failed"
            reasons.append(f"missing positive line_no for {item['name']}")

    if not report or report.get("status") != "console_emitted":
        status = "failed"
        reasons.append("page did not report console_emitted")

    return {
        "status": status,
        "reasons": reasons,
        "expected": expected,
        "matches": matches,
    }


def verify_http_auth_probe(
    probe_name: str,
    tab_id: int | None,
    report: dict[str, Any] | None,
    reports: list[dict[str, Any]],
    http_auth: list[dict[str, Any]],
    auth_events: list[dict[str, Any]],
    log_text: str,
) -> dict[str, Any] | None:
    if probe_name not in ("http-basic-auth-success", "http-basic-auth-cancel"):
        return None

    status = "completed"
    reasons: list[str] = []
    request = http_auth[0] if http_auth else None
    if not request:
        status = "failed"
        reasons.append("missing HttpAuthRequest")
    else:
        if tab_id is not None and request.get("tab_id") != tab_id:
            status = "failed"
            reasons.append(f"wrong tab_id: {request.get('tab_id')}")
        if request.get("auth_scheme") != "basic":
            status = "failed"
            reasons.append(f"wrong auth_scheme: {request.get('auth_scheme')}")
        if HTTP_AUTH_REALM != request.get("realm"):
            status = "failed"
            reasons.append(f"wrong realm: {request.get('realm')}")
        if request.get("is_proxy"):
            status = "failed"
            reasons.append("request unexpectedly marked proxy")
        if not str(request.get("challenger", "")).startswith("http://"):
            status = "failed"
            reasons.append(f"challenger missing serialized origin: {request.get('challenger')}")
        if request.get("reply_request_id") != request.get("request_id"):
            status = "failed"
            reasons.append("reply request_id did not match request")

    event_statuses = [event.get("status") for event in auth_events]
    credential_pair = f"{HTTP_AUTH_USERNAME}:{HTTP_AUTH_PASSWORD}"
    encoded_credential = base64.b64encode(credential_pair.encode("utf-8")).decode("ascii")
    leaked = [
        needle
        for needle in (
            HTTP_AUTH_PASSWORD,
            credential_pair,
            encoded_credential,
            "Authorization: Basic",
        )
        if needle in log_text
    ]
    if leaked:
        status = "failed"
        reasons.append(f"credential material leaked to logs: {leaked}")

    if "challenge" not in event_statuses:
        status = "failed"
        reasons.append("server did not record 401 challenge")

    if probe_name == "http-basic-auth-success":
        if not report or report.get("status") != "auth_success":
            status = "failed"
            reasons.append("protected page did not report auth_success")
        if "authorized" not in event_statuses:
            status = "failed"
            reasons.append("server did not record authorized request")
        if request:
            challenge_event = next(
                (event for event in auth_events if event.get("status") == "challenge"),
                None,
            )
            authorized_event = next(
                (event for event in auth_events if event.get("status") == "authorized"),
                None,
            )
            reply_at = request.get("reply_received_at")
            if not (challenge_event and authorized_event and reply_at):
                status = "failed"
                reasons.append("missing auth sequence timing evidence")
            elif not (
                challenge_event.get("_received_at", 0)
                <= float(reply_at)
                <= authorized_event.get("_received_at", 0)
            ):
                status = "failed"
                reasons.append("auth server/protocol sequence was not ordered")
    else:
        if report and report.get("status") == "auth_success":
            status = "failed"
            reasons.append("cancel probe reached protected auth_success page")
        if not any(r.get("status") == "post_cancel_navigation" for r in reports):
            status = "failed"
            reasons.append("post-cancel navigation did not report completion")
        if "authorized" in event_statuses:
            status = "failed"
            reasons.append("cancel probe unexpectedly sent valid credentials")

    return {
        "status": status,
        "reasons": reasons,
        "messages": http_auth,
        "server_events": auth_events,
    }


def verify_renderer_crash_probe(
    probe_name: str,
    tab_id: int | None,
    reports: list[dict[str, Any]],
    loading_states: list[dict[str, Any]],
    renderer_crashes: list[dict[str, Any]],
    crash_navigation_sent: bool,
    post_crash_navigation_sent: bool,
    socket_disconnect: bool,
    proc_exit: int | None,
) -> dict[str, Any] | None:
    if probe_name != "renderer-crash-recovery":
        return None

    status = "completed"
    reasons: list[str] = []
    crash = renderer_crashes[0] if renderer_crashes else None
    report_statuses = [str(report.get("status")) for report in reports]

    if "ready" not in report_statuses:
        status = "failed"
        reasons.append("initial page did not report ready")
    if not crash_navigation_sent:
        status = "failed"
        reasons.append("chrome://crash navigation was not sent")
    if not crash:
        status = "failed"
        reasons.append("missing RendererCrashed event")
    else:
        if tab_id is not None and crash.get("tab_id") != tab_id:
            status = "failed"
            reasons.append(f"wrong tab_id: {crash.get('tab_id')}")
        if crash.get("termination_status") in (
            "",
            "normal_termination",
            "still_running",
        ):
            status = "failed"
            reasons.append(f"non-crash status: {crash.get('termination_status')}")
        if int(crash.get("termination_status_code", 0) or 0) == 0:
            status = "failed"
            reasons.append("missing nonzero termination_status_code")
        if "chrome://crash" not in str(crash.get("url", "")):
            status = "failed"
            reasons.append(f"crash URL missing from event: {crash.get('url')}")
        if not crash.get("can_reload"):
            status = "failed"
            reasons.append("RendererCrashed can_reload was false")
    if len(renderer_crashes) != 1:
        status = "failed"
        reasons.append(f"expected one RendererCrashed event, got {len(renderer_crashes)}")
    if not any(state.get("state") == "error" for state in loading_states):
        status = "failed"
        reasons.append("missing LoadingState error")
    if not post_crash_navigation_sent:
        status = "failed"
        reasons.append("post-crash navigation was not sent")
    if "post_crash_navigation" not in report_statuses:
        status = "failed"
        reasons.append("post-crash page did not report completion")
    if socket_disconnect:
        status = "failed"
        reasons.append("socket disconnected before recovery completed")
    if proc_exit is not None and proc_exit not in (0, -15):
        status = "failed"
        reasons.append(f"Roamium exited unexpectedly: {proc_exit}")

    return {
        "status": status,
        "reasons": reasons,
        "messages": renderer_crashes,
        "loading_states": loading_states,
        "reports": reports,
    }


def create_tab_payload(url: str, width: int, height: int) -> bytes:
    return (
        string_field(1, url)
        + string_field(2, "issue-799-fake-pane")
        + varint_field(3, width)
        + varint_field(4, height)
        + bool_field(5, False)
    )


def resize_payload(tab_id: int, width: int, height: int) -> bytes:
    return varint_field(1, tab_id) + varint_field(2, width) + varint_field(3, height)


def focus_changed_payload(tab_id: int, focused: bool) -> bytes:
    return varint_field(1, tab_id) + bool_field(2, focused)


def navigate_payload(tab_id: int, url: str) -> bytes:
    return varint_field(1, tab_id) + string_field(3, url)


def mouse_move_payload(tab_id: int, x: int, y: int) -> bytes:
    return varint_field(1, tab_id) + fixed_double_field(2, x) + fixed_double_field(3, y)


def mouse_event_payload(tab_id: int, event_type: str, x: int, y: int) -> bytes:
    return (
        varint_field(1, tab_id)
        + string_field(2, event_type)
        + string_field(3, "left")
        + fixed_double_field(4, x)
        + fixed_double_field(5, y)
        + varint_field(6, 1)
    )


def key_event_payload(
    tab_id: int,
    event_type: str,
    keycode: int,
    utf8: str = "",
    modifiers: int = 0,
) -> bytes:
    return (
        varint_field(1, tab_id)
        + string_field(2, event_type)
        + varint_field(3, keycode)
        + string_field(4, utf8)
        + varint_field(5, modifiers)
    )


def send_key_pair(
    conn: socket.socket,
    tab_id: int,
    keycode: int,
    utf8: str = "",
    modifiers: int = 0,
) -> None:
    send_message(conn, 9, key_event_payload(tab_id, "down", keycode, utf8, modifiers))
    send_message(conn, 9, key_event_payload(tab_id, "up", keycode, "", modifiers))


class ProbeState:
    def __init__(self, run_dir: pathlib.Path) -> None:
        self.run_dir = run_dir
        self.lock = threading.Lock()
        self.reports: list[dict[str, Any]] = []
        self.auth_events: list[dict[str, Any]] = []

    def add_report(self, report: dict[str, Any]) -> None:
        with self.lock:
            report["_received_at"] = time.time()
            self.reports.append(report)
            with (self.run_dir / "reports.jsonl").open("a", encoding="utf-8") as out:
                out.write(json.dumps(report, sort_keys=True) + "\n")

    def reports_for(self, probe: str) -> list[dict[str, Any]]:
        with self.lock:
            return [report for report in self.reports if report.get("probe") == probe]

    def add_auth_event(self, event: dict[str, Any]) -> None:
        with self.lock:
            event["_received_at"] = time.time()
            self.auth_events.append(event)
            with (self.run_dir / "auth-events.jsonl").open("a", encoding="utf-8") as out:
                out.write(json.dumps(event, sort_keys=True) + "\n")

    def auth_events_for(self, probe: str) -> list[dict[str, Any]]:
        with self.lock:
            return [event for event in self.auth_events if event.get("probe") == probe]


class ReusableThreadingTcpServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def make_handler(state: ProbeState) -> type[http.server.BaseHTTPRequestHandler]:
    probe_by_name = {probe.name: probe for probe in PROBES}

    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, fmt: str, *args: object) -> None:
            with (state.run_dir / "http.log").open("a", encoding="utf-8") as log:
                log.write((fmt % args) + "\n")

        def do_GET(self) -> None:
            parsed = urlparse(self.path)
            if parsed.path == "/auth/basic/success.html":
                query = parse_qs(parsed.query)
                probe = query.get("probe", ["unknown"])[-1]
                nonce = query.get("nonce", [HTTP_AUTH_NONCE])[-1]
                expected = "Basic " + base64.b64encode(
                    f"{HTTP_AUTH_USERNAME}:{HTTP_AUTH_PASSWORD}".encode("utf-8")
                ).decode("ascii")
                if self.headers.get("Authorization") != expected:
                    state.add_auth_event(
                        {
                            "probe": probe,
                            "status": "challenge",
                            "path": parsed.path,
                            "has_authorization": bool(self.headers.get("Authorization")),
                        }
                    )
                    self.send_response(401)
                    self.send_header(
                        "WWW-Authenticate",
                        f'Basic realm="{HTTP_AUTH_REALM}"',
                    )
                    self.send_header("Content-Type", "text/plain; charset=utf-8")
                    self.end_headers()
                    self.wfile.write(b"authentication required")
                    return

                state.add_auth_event(
                    {
                        "probe": probe,
                        "status": "authorized",
                        "path": parsed.path,
                    }
                )
                body = f"""<!doctype html>
<meta charset="utf-8">
<title>auth success</title>
<script>
fetch('/report', {{
  method: 'POST',
  headers: {{'Content-Type': 'application/json'}},
  body: JSON.stringify({{
    probe: {json.dumps(probe)},
    status: 'auth_success',
    nonce: {json.dumps(nonce)},
    reportedAt: new Date().toISOString()
  }}),
  keepalive: true
}});
</script>
authenticated
""".encode("utf-8")
                self.send_bytes(body, "text/html; charset=utf-8")
                return
            if parsed.path == "/auth/post-cancel.html":
                query = parse_qs(parsed.query)
                probe = query.get("probe", ["unknown"])[-1]
                state.add_auth_event(
                    {
                        "probe": probe,
                        "status": "post_cancel_navigation",
                        "path": parsed.path,
                    }
                )
                body = f"""<!doctype html>
<meta charset="utf-8">
<title>post cancel</title>
<script>
fetch('/report', {{
  method: 'POST',
  headers: {{'Content-Type': 'application/json'}},
  body: JSON.stringify({{
    probe: {json.dumps(probe)},
    status: 'post_cancel_navigation',
    reportedAt: new Date().toISOString()
  }}),
  keepalive: true
}});
</script>
post cancel
""".encode("utf-8")
                self.send_bytes(body, "text/html; charset=utf-8")
                return
            if parsed.path == "/renderer-crash/post-crash.html":
                query = parse_qs(parsed.query)
                probe = query.get("probe", ["unknown"])[-1]
                body = f"""<!doctype html>
<meta charset="utf-8">
<title>post crash recovery</title>
<script>
fetch('/report', {{
  method: 'POST',
  headers: {{'Content-Type': 'application/json'}},
  body: JSON.stringify({{
    probe: {json.dumps(probe)},
    status: 'post_crash_navigation',
    reportedAt: new Date().toISOString()
  }}),
  keepalive: true
}});
</script>
post crash
""".encode("utf-8")
                self.send_bytes(body, "text/html; charset=utf-8")
                return
            if parsed.path == "/probe/console-capture-basic-frame.html":
                query = parse_qs(parsed.query)
                nonce = query.get("nonce", ["issue799"])[-1]
                body = f"""<!doctype html>
<meta charset="utf-8">
<title>console frame</title>
<script>
console.warn({json.dumps("ts-console-frame-warn-" + nonce)});
</script>
frame
""".encode("utf-8")
                self.send_bytes(body, "text/html; charset=utf-8")
                return
            if parsed.path.startswith("/probe/") and parsed.path.endswith(".html"):
                name = pathlib.PurePosixPath(parsed.path).stem
                probe = probe_by_name.get(name)
                if not probe:
                    self.send_error(404)
                    return
                self.send_bytes(render_probe_page(probe), "text/html; charset=utf-8")
                return
            if parsed.path == "/probe/service-worker-basic-worker.js":
                self.send_bytes(
                    b"self.addEventListener('install', event => self.skipWaiting());\n"
                    b"self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));\n",
                    "application/javascript; charset=utf-8",
                )
                return
            if parsed.path == "/download/attachment.txt":
                self.send_response(200)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.send_header(
                    "Content-Disposition",
                    'attachment; filename="termsurf-download.txt"',
                )
                self.send_header("Content-Length", str(len(ATTACHMENT_DOWNLOAD_BYTES)))
                self.end_headers()
                self.wfile.write(ATTACHMENT_DOWNLOAD_BYTES)
                return
            if parsed.path == "/beforeunload-destination.html":
                query = parse_qs(parsed.query)
                probe = query.get("probe", ["unknown"])[-1]
                body = f"""<!doctype html>
<meta charset="utf-8">
<title>beforeunload destination</title>
<script>
fetch('/report', {{
  method: 'POST',
  headers: {{'Content-Type': 'application/json'}},
  body: JSON.stringify({{
    probe: {json.dumps(probe)},
    status: 'destination_loaded',
    reportedAt: new Date().toISOString()
  }}),
  keepalive: true
}});
</script>
destination
""".encode("utf-8")
                self.send_bytes(body, "text/html; charset=utf-8")
                return
            if parsed.path == "/summary":
                self.send_bytes(
                    json.dumps(state.reports, indent=2).encode("utf-8"),
                    "application/json; charset=utf-8",
                )
                return
            if parsed.path == "/report":
                query = parse_qs(parsed.query)
                report = {key: values[-1] for key, values in query.items()}
                state.add_report(report)
                self.send_bytes(b"ok\n", "text/plain; charset=utf-8")
                return
            self.send_error(404)

        def do_POST(self) -> None:
            parsed = urlparse(self.path)
            if parsed.path != "/report":
                self.send_error(404)
                return
            length = int(self.headers.get("Content-Length", "0") or "0")
            data = self.rfile.read(length)
            try:
                report = json.loads(data.decode("utf-8"))
            except json.JSONDecodeError:
                report = {"parse_error": data.decode("utf-8", errors="replace")}
            state.add_report(report)
            self.send_bytes(b"ok\n", "text/plain; charset=utf-8")

        def send_bytes(self, data: bytes, content_type: str) -> None:
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    return Handler


def render_probe_page(probe: Probe) -> bytes:
    title = html.escape(probe.name)
    timeout_ms = 12000 if probe.name == "webauthn-create" else 5000
    script = f"""
const probeName = {json.dumps(probe.name)};
async function sendReport(report) {{
  report.probe = probeName;
  report.reportedAt = new Date().toISOString();
  try {{
    await fetch('/report', {{
      method: 'POST',
      headers: {{'Content-Type': 'application/json'}},
      body: JSON.stringify(report),
      keepalive: true
    }});
  }} catch (error) {{
    new Image().src = '/report?probe=' + encodeURIComponent(probeName) +
      '&status=report_failed&error=' + encodeURIComponent(String(error));
  }}
}}
let completed = false;
let timeoutId = null;
async function finalReport(report) {{
  if (completed) return;
  completed = true;
  if (timeoutId !== null) clearTimeout(timeoutId);
  await sendReport(report);
}}
async function runProbe() {{
  try {{
    const detail = await (async () => {{
      {probe.script}
    }})();
    await finalReport({{ok: true, ...(detail || {{status: 'resolved'}})}});
  }} catch (error) {{
    await finalReport({{
      ok: false,
      status: 'exception',
      error: String(error),
      errorName: error?.name || null,
      stack: error?.stack || null
    }});
  }}
}}
timeoutId = setTimeout(() => finalReport({{ok: false, status: 'page_timeout'}}), {timeout_ms});
runProbe();
"""
    return f"""<!doctype html>
<meta charset="utf-8">
<title>Issue 799 probe: {title}</title>
<h1>Issue 799 probe: {title}</h1>
<script>
{script}
</script>
""".encode("utf-8")


def start_server(state: ProbeState) -> ReusableThreadingTcpServer:
    server = ReusableThreadingTcpServer(("127.0.0.1", 0), make_handler(state))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server


def timestamp() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y%m%d-%H%M%S-%f")


def write_json(path: pathlib.Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def read_text(path: pathlib.Path) -> str:
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def devtools_port_from_log(path: pathlib.Path) -> int | None:
    matches = DEVTOOLS_RE.findall(read_text(path))
    if not matches:
        return None
    return int(matches[-1])


def start_webauthn_devtools_setup(
    probe_name: str,
    probe_dir: pathlib.Path,
    stderr_path: pathlib.Path,
) -> tuple[dict[str, Any], subprocess.Popen[str] | None]:
    out_path = probe_dir / "webauthn-devtools.json"
    stdout_path = probe_dir / "webauthn-devtools.stdout"
    stderr_helper_path = probe_dir / "webauthn-devtools.stderr"
    port = devtools_port_from_log(stderr_path)
    result: dict[str, Any] = {
        "status": "missing_devtools_port",
        "devtools_port": port,
        "artifact_path": str(out_path),
        "stdout_path": str(stdout_path),
        "stderr_path": str(stderr_helper_path),
    }
    if port is None:
        write_json(out_path, result)
        return result, None

    helper = ROOT / "scripts/issue-799-webauthn-virtual-authenticator.mjs"
    command = [
        "node",
        str(helper),
        "--devtools-port",
        str(port),
        "--url-contains",
        f"/probe/{probe_name}.html",
        "--out",
        str(out_path),
        "--timeout-seconds",
        "6",
        "--hold-seconds",
        "20",
    ]
    stdout = stdout_path.open("w", encoding="utf-8")
    stderr = stderr_helper_path.open("w", encoding="utf-8")
    proc = subprocess.Popen(
        command,
        cwd=str(ROOT),
        stdout=stdout,
        stderr=stderr,
        text=True,
    )
    deadline = time.time() + 8
    artifact: dict[str, Any] = {}
    while time.time() < deadline:
        artifact = json.loads(read_text(out_path) or "{}")
        if artifact.get("status") in ("completed", "failed"):
            break
        if proc.poll() is not None:
            break
        time.sleep(0.1)
    stdout.close()
    stderr.close()
    artifact = json.loads(read_text(out_path) or "{}")
    if not artifact:
        result.update(
            {
                "status": "timeout",
                "devtools_port": port,
            }
        )
        write_json(out_path, result)
        return result, proc
    result.update(
        {
            "status": artifact.get("status", "failed"),
            "returncode": proc.poll(),
            "stdout": read_text(stdout_path),
            "stderr": read_text(stderr_helper_path),
            "artifact": artifact,
            "devtools_port": port,
        }
    )
    return result, proc


def scan_logs(text: str, *, expect_renderer_crash: bool = False) -> dict[str, Any]:
    missing = sorted(set(MISSING_INTERFACE_RE.findall(text)))
    empty = sorted(set(EMPTY_BINDER_RE.findall(text)))
    text_lines = text.splitlines()
    bad_mojo_lines = [
        line
        for line in text_lines
        if any(pattern in line for pattern in BAD_MOJO_PATTERNS)
    ]
    empty_binder_lines = [
        line for line in text_lines if "Empty binder for interface" in line
    ]
    crash_entries = [
        (index, line)
        for index, line in enumerate(text_lines)
        if any(pattern in line for pattern in CRASH_PATTERNS)
    ]
    expected_crash_indexes: set[int] = set()
    if expect_renderer_crash:
        for index, line in enumerate(text_lines):
            if (
                "third_party/blink/common/chrome_debug_urls.cc" in line
                and "Intentionally crashing" in line
                and "chrome://crash/" in line
                and "inducebrowsercrashforrealz" not in line
            ):
                for candidate in range(index + 1, min(index + 12, len(text_lines))):
                    candidate_line = text_lines[candidate]
                    if "[termsurf-renderer-crash]" in candidate_line:
                        break
                    if any(pattern in candidate_line for pattern in CRASH_PATTERNS):
                        expected_crash_indexes.add(candidate)
    crash_lines = [line for _, line in crash_entries]
    expected_crash_lines = [
        line for index, line in crash_entries if index in expected_crash_indexes
    ]
    unexpected_crash_lines = [
        line for index, line in crash_entries if index not in expected_crash_indexes
    ]
    return {
        "bad_mojo": bool(bad_mojo_lines),
        "crashed": bool(unexpected_crash_lines),
        "missing_interfaces": missing,
        "empty_interfaces": empty,
        "bad_mojo_lines": bad_mojo_lines,
        "empty_binder_lines": empty_binder_lines,
        "crash_lines": crash_lines,
        "expected_crash_lines": expected_crash_lines,
        "unexpected_crash_lines": unexpected_crash_lines,
    }


def classify_probe(report: dict[str, Any] | None, log_scan: dict[str, Any], proc_exit: int | None) -> str:
    if log_scan["missing_interfaces"]:
        return "missing_binder"
    if log_scan["bad_mojo"]:
        return "bad_mojo"
    if log_scan["crashed"]:
        return "renderer_or_browser_crash"
    if proc_exit is not None and proc_exit not in (0, -15):
        return "process_exit"
    if not report:
        return "no_report"
    status = str(report.get("status", "unknown"))
    error_text = str(report.get("error", ""))
    if "IPC connection" in error_text or "service in the browser process" in error_text:
        if log_scan["empty_interfaces"]:
            return "empty_binder"
        return "browser_service_unavailable"
    if status == "blocked_user_activation":
        return "blocked_user_activation"
    if status == "blocked_needs_virtual_authenticator":
        return "blocked_needs_virtual_authenticator"
    if status == "unsupported":
        return "unsupported"
    if status in ("resolved", "rejected"):
        return "exercised"
    if status == "exception":
        return "js_exception"
    if status == "page_timeout":
        return "page_timeout"
    if status == "probe_timeout":
        return "probe_timeout"
    return "reported"


def verify_download(
    download_dir: pathlib.Path,
    filename: str,
    expected_bytes: bytes,
    deadline: float,
) -> dict[str, Any]:
    target = download_dir / filename
    intermediate = download_dir / f"{filename}.crdownload"
    last_state = "missing"
    while time.time() < deadline:
        crdownloads = sorted(path.name for path in download_dir.glob("*.crdownload"))
        if target.exists() and not intermediate.exists() and not crdownloads:
            actual = target.read_bytes()
            actual_hash = hashlib.sha256(actual).hexdigest()
            expected_hash = hashlib.sha256(expected_bytes).hexdigest()
            return {
                "status": "completed" if actual == expected_bytes else "wrong_bytes",
                "path": str(target),
                "filename": filename,
                "size": len(actual),
                "sha256": actual_hash,
                "expected_size": len(expected_bytes),
                "expected_sha256": expected_hash,
            }
        if target.exists():
            last_state = "waiting_for_intermediate"
        elif crdownloads:
            last_state = "intermediate_only"
        time.sleep(0.1)
    crdownloads = sorted(path.name for path in download_dir.glob("*.crdownload"))
    return {
        "status": "timeout",
        "path": str(target),
        "filename": filename,
        "exists": target.exists(),
        "last_state": last_state,
        "crdownloads": crdownloads,
    }


def run_probe(
    probe: Probe,
    base_url: str,
    state: ProbeState,
    run_dir: pathlib.Path,
    roamium: pathlib.Path,
    download_dir: pathlib.Path,
    seconds: float,
    width: int,
    height: int,
) -> dict[str, Any]:
    if probe.name == "webauthn-create":
        seconds = max(seconds, 16.0)
    probe_dir = run_dir / "probes" / probe.name
    probe_dir.mkdir(parents=True, exist_ok=True)
    socket_path = (
        pathlib.Path(tempfile.gettempdir())
        / f"ts799-{os.getpid()}-{hashlib.sha1(probe.name.encode()).hexdigest()[:8]}.sock"
    )
    try:
        socket_path.unlink()
    except FileNotFoundError:
        pass

    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(socket_path))
    listener.listen(1)
    listener.settimeout(min(20.0, seconds))

    stdout_path = probe_dir / "roamium.stdout"
    stderr_path = probe_dir / "roamium.stderr"
    messages_path = probe_dir / "messages.log"
    stdout = stdout_path.open("wb")
    stderr = stderr_path.open("wb")
    if probe.name.startswith("http-basic-auth-"):
        url = (
            f"{base_url}/auth/basic/success.html?"
            f"probe={probe.name}&nonce={HTTP_AUTH_NONCE}"
        )
    else:
        url = f"{base_url}/probe/{probe.name}.html"
    expected_download = EXPECTED_DOWNLOADS.get(probe.name)
    if expected_download:
        expected_path = download_dir / expected_download[0]
        expected_path.unlink(missing_ok=True)
        expected_path.with_name(expected_path.name + ".crdownload").unlink(
            missing_ok=True
        )
    command = [
        str(roamium),
        f"--ipc-socket={socket_path}",
        f"--user-data-dir={probe_dir / 'profile'}",
        f"--termsurf-download-dir={download_dir}",
        "--no-sandbox",
        "--enable-logging=stderr",
    ]
    env = os.environ.copy()
    env["TERMSURF_PDF_INPUT_TRACE"] = "1"
    env["TERMSURF_PDF_INPUT_TRACE_FILE"] = str(probe_dir / "input-trace.log")
    proc = subprocess.Popen(
        command,
        cwd=str(ROOT / "chromium/src"),
        stdout=stdout,
        stderr=stderr,
        env=env,
    )

    sent_create = False
    tab_id: int | None = None
    socket_disconnect = False
    javascript_dialogs: list[dict[str, Any]] = []
    console_messages: list[dict[str, Any]] = []
    http_auth: list[dict[str, Any]] = []
    loading_states: list[dict[str, Any]] = []
    renderer_crashes: list[dict[str, Any]] = []
    activation_sent = False
    activation_ready_at: float | None = None
    activation_sent_at: float | None = None
    activation_observed_at: float | None = None
    picker_call_started_at: float | None = None
    navigation_sent = False
    page_zoom_events: list[dict[str, Any]] = []
    page_zoom_next_step = 0
    http_auth_cancel_sent_at: float | None = None
    post_cancel_navigation_sent = False
    crash_navigation_sent = False
    post_crash_navigation_sent = False
    webauthn_devtools_setup: dict[str, Any] | None = None
    webauthn_devtools_process: subprocess.Popen[str] | None = None
    start = time.time()

    try:
        try:
            conn, _ = listener.accept()
            conn.settimeout(0.2)
        except socket.timeout:
            conn = None
        with messages_path.open("w", encoding="utf-8") as messages:
            while time.time() - start < seconds:
                if proc.poll() is not None:
                    break
                if conn is None:
                    time.sleep(0.1)
                    continue
                if probe.name.startswith("javascript-beforeunload-") and tab_id:
                    reports = state_reports_for_probe(run_dir, probe.name)
                    statuses = {str(report.get("status")) for report in reports}
                    if "ready" in statuses and activation_ready_at is None:
                        activation_ready_at = time.time()
                    if "activated" in statuses and activation_observed_at is None:
                        activation_observed_at = time.time()
                    if (
                        activation_ready_at is not None
                        and time.time() - activation_ready_at >= 0.5
                        and not activation_sent
                    ):
                        send_message(conn, 10, focus_changed_payload(tab_id, True))
                        send_message(conn, 7, mouse_move_payload(tab_id, 20, 20))
                        send_message(conn, 6, mouse_event_payload(tab_id, "down", 20, 20))
                        send_message(conn, 6, mouse_event_payload(tab_id, "up", 20, 20))
                        send_message(conn, 9, key_event_payload(tab_id, "down", 65, "a"))
                        send_message(conn, 9, key_event_payload(tab_id, "up", 65))
                        activation_sent = True
                        activation_sent_at = time.time()
                        messages.write("sent beforeunload refocus and activation input\n")
                        messages.flush()
                    if activation_observed_at is not None and not navigation_sent:
                        destination = (
                            f"{base_url}/beforeunload-destination.html?probe={probe.name}"
                        )
                        send_message(conn, 5, navigate_payload(tab_id, destination))
                        navigation_sent = True
                        messages.write(f"sent beforeunload Navigate url={destination}\n")
                        messages.flush()
                if probe.name == "file-system-access" and tab_id:
                    reports = state_reports_for_probe(run_dir, probe.name)
                    statuses = {str(report.get("status")) for report in reports}
                    if "ready" in statuses and activation_ready_at is None:
                        activation_ready_at = time.time()
                    if "activated" in statuses and activation_observed_at is None:
                        activation_observed_at = time.time()
                    if (
                        "picker_call_started_after_activation" in statuses
                        and picker_call_started_at is None
                    ):
                        picker_call_started_at = time.time()
                    if (
                        activation_ready_at is not None
                        and time.time() - activation_ready_at >= 0.5
                        and not activation_sent
                    ):
                        send_message(conn, 10, focus_changed_payload(tab_id, True))
                        send_message(conn, 7, mouse_move_payload(tab_id, 20, 20))
                        send_message(conn, 6, mouse_event_payload(tab_id, "down", 20, 20))
                        send_message(conn, 6, mouse_event_payload(tab_id, "up", 20, 20))
                        activation_sent = True
                        activation_sent_at = time.time()
                        messages.write("sent file-system-access activation input\n")
                        messages.flush()
                if probe.name == "webauthn-create" and tab_id:
                    reports = state_reports_for_probe(run_dir, probe.name)
                    statuses = {str(report.get("status")) for report in reports}
                    if "ready" in statuses and activation_ready_at is None:
                        activation_ready_at = time.time()
                    if "activated" in statuses and activation_observed_at is None:
                        activation_observed_at = time.time()
                    if activation_ready_at is not None and webauthn_devtools_setup is None:
                        if devtools_port_from_log(stderr_path) is not None:
                            (
                                webauthn_devtools_setup,
                                webauthn_devtools_process,
                            ) = start_webauthn_devtools_setup(
                                probe.name, probe_dir, stderr_path
                            )
                            messages.write(
                                "webauthn devtools setup "
                                f"status={webauthn_devtools_setup.get('status')} "
                                f"port={webauthn_devtools_setup.get('devtools_port')}\n"
                            )
                            messages.flush()
                    if (
                        webauthn_devtools_setup is not None
                        and webauthn_devtools_setup.get("status") == "completed"
                        and not activation_sent
                    ):
                        send_message(conn, 10, focus_changed_payload(tab_id, True))
                        send_message(conn, 7, mouse_move_payload(tab_id, 20, 20))
                        send_message(conn, 6, mouse_event_payload(tab_id, "down", 20, 20))
                        send_message(conn, 6, mouse_event_payload(tab_id, "up", 20, 20))
                        activation_sent = True
                        activation_sent_at = time.time()
                        messages.write("sent webauthn activation input\n")
                        messages.flush()
                if probe.name == "page-zoom-shortcuts" and tab_id:
                    reports = state_reports_for_probe(run_dir, probe.name)
                    metric_reports = [
                        report
                        for report in reports
                        if report.get("status") == "page_zoom_metrics"
                    ]
                    has_baseline = any(
                        report.get("label") == "baseline" for report in metric_reports
                    )

                    def send_zoom_key(name: str, keycode: int, modifiers: int) -> None:
                        nonlocal page_zoom_next_step
                        send_key_pair(conn, tab_id or 0, keycode, modifiers=modifiers)
                        sent_at = time.time()
                        page_zoom_events.append(
                            {
                                "name": name,
                                "keycode": keycode,
                                "modifiers": modifiers,
                                "sent_at": sent_at,
                                "elapsed": sent_at - start,
                            }
                        )
                        page_zoom_next_step += 1
                        messages.write(
                            f"sent page zoom key name={name} "
                            f"keycode={keycode} modifiers={modifiers}\n"
                        )
                        messages.flush()

                    if page_zoom_next_step == 0 and has_baseline:
                        send_zoom_key(
                            "zoom-in", VK_OEM_PLUS, TERMSURF_META_MODIFIER
                        )
                    elif page_zoom_next_step == 1:
                        sent_at = page_zoom_events[-1]["sent_at"]
                        metric_after = first_metric_after(metric_reports, sent_at)
                        if metric_after or time.time() - sent_at > 1.2:
                            send_zoom_key(
                                "zoom-out",
                                VK_OEM_MINUS,
                                TERMSURF_META_MODIFIER,
                            )
                    elif page_zoom_next_step == 2:
                        sent_at = page_zoom_events[-1]["sent_at"]
                        metric_after = first_metric_after(metric_reports, sent_at)
                        if metric_after or time.time() - sent_at > 1.2:
                            send_zoom_key("reset", VK_0, TERMSURF_META_MODIFIER)
                    elif page_zoom_next_step == 3:
                        sent_at = page_zoom_events[-1]["sent_at"]
                        metric_after = first_metric_after(metric_reports, sent_at)
                        if metric_after or time.time() - sent_at > 1.2:
                            send_zoom_key("normal-a", VK_A, 0)
                if (
                    probe.name == "http-basic-auth-cancel"
                    and tab_id
                    and http_auth_cancel_sent_at is not None
                    and not post_cancel_navigation_sent
                    and time.time() - http_auth_cancel_sent_at >= 0.5
                ):
                    destination = f"{base_url}/auth/post-cancel.html?probe={probe.name}"
                    send_message(conn, 5, navigate_payload(tab_id, destination))
                    post_cancel_navigation_sent = True
                    messages.write(f"sent post-cancel Navigate url={destination}\n")
                    messages.flush()
                if probe.name == "renderer-crash-recovery" and tab_id:
                    reports = state_reports_for_probe(run_dir, probe.name)
                    statuses = {str(report.get("status")) for report in reports}
                    if "ready" in statuses and not crash_navigation_sent:
                        send_message(conn, 5, navigate_payload(tab_id, "chrome://crash/"))
                        crash_navigation_sent = True
                        messages.write("sent renderer crash Navigate url=chrome://crash/\n")
                        messages.flush()
                    has_error = any(
                        state.get("state") == "error" for state in loading_states
                    )
                    if renderer_crashes and has_error and not post_crash_navigation_sent:
                        destination = (
                            f"{base_url}/renderer-crash/post-crash.html?probe={probe.name}"
                        )
                        send_message(conn, 5, navigate_payload(tab_id, destination))
                        post_crash_navigation_sent = True
                        messages.write(f"sent post-crash Navigate url={destination}\n")
                        messages.flush()
                try:
                    header = conn.recv(4)
                    if not header:
                        socket_disconnect = True
                        break
                    size = struct.unpack("<I", header)[0]
                    payload = bytearray()
                    while len(payload) < size:
                        chunk = conn.recv(size - len(payload))
                        if not chunk:
                            socket_disconnect = True
                            break
                        payload.extend(chunk)
                    if socket_disconnect:
                        break
                    top, body = inner_payload(bytes(payload))
                    messages.write(f"t={time.time() - start:.3f} top_field={top}\n")
                    messages.flush()
                    if top == 12 and not sent_create:
                        send_message(conn, 1, create_tab_payload(url, width, height))
                        sent_create = True
                        messages.write(f"sent CreateTab url={url}\n")
                        messages.flush()
                    elif top == 13:
                        tab_id = tab_ready_id(body)
                        messages.write(f"tab_ready id={tab_id}\n")
                        if tab_id:
                            send_message(conn, 3, resize_payload(tab_id, width, height))
                            send_message(conn, 10, focus_changed_payload(tab_id, True))
                            messages.write("sent Resize\n")
                            messages.write("sent FocusChanged focused=true\n")
                        messages.flush()
                    elif top == 16:
                        fields = parse_message_fields(body)
                        evidence = {
                            "received_time": time.time() - start,
                            "tab_id": int(fields.get(1, 0) or 0),
                            "state": str(fields.get(2, "")),
                            "progress": int(fields.get(3, 0) or 0),
                        }
                        loading_states.append(evidence)
                        messages.write(
                            "loading_state "
                            f"state={evidence['state']} "
                            f"progress={evidence['progress']}\n"
                        )
                        messages.flush()
                    elif top == 34:
                        fields = parse_message_fields(body)
                        request_tab_id = int(fields.get(1, 0) or 0)
                        request_id = int(fields.get(2, 0) or 0)
                        dialog_type = str(fields.get(3, ""))
                        origin_url = str(fields.get(4, ""))
                        message = str(fields.get(5, ""))
                        default_prompt_text = str(fields.get(6, ""))
                        accepted, prompt_text = dialog_response_for(probe.name)
                        send_message(
                            conn,
                            35,
                            javascript_dialog_reply_payload(
                                request_tab_id,
                                request_id,
                                accepted,
                                prompt_text,
                            ),
                        )
                        evidence = {
                            "request_time": time.time() - start,
                            "tab_id": request_tab_id,
                            "request_id": request_id,
                            "dialog_type": dialog_type,
                            "origin_url": origin_url,
                            "message": message,
                            "default_prompt_text": default_prompt_text,
                            "accepted": accepted,
                            "prompt_text": prompt_text,
                            "reply_time": time.time() - start,
                        }
                        javascript_dialogs.append(evidence)
                        messages.write(
                            "javascript_dialog "
                            f"type={dialog_type} request_id={request_id} "
                            f"accepted={accepted}\n"
                        )
                        messages.flush()
                    elif top == 36:
                        fields = parse_message_fields(body)
                        evidence = {
                            "received_time": time.time() - start,
                            "tab_id": int(fields.get(1, 0) or 0),
                            "level": str(fields.get(2, "")),
                            "message": str(fields.get(3, "")),
                            "line_no": int(fields.get(4, 0) or 0),
                            "source_id": str(fields.get(5, "")),
                        }
                        console_messages.append(evidence)
                        messages.write(
                            "console_message "
                            f"level={evidence['level']} "
                            f"line_no={evidence['line_no']} "
                            f"source={evidence['source_id']}\n"
                        )
                        messages.flush()
                    elif top == 37:
                        fields = parse_message_fields(body)
                        request_tab_id = int(fields.get(1, 0) or 0)
                        request_id = int(fields.get(2, 0) or 0)
                        url_value = str(fields.get(3, ""))
                        auth_scheme = str(fields.get(4, ""))
                        challenger = str(fields.get(5, ""))
                        realm = str(fields.get(6, ""))
                        is_proxy = bool(fields.get(7, False))
                        first_auth_attempt = bool(fields.get(8, False))
                        accepted = probe.name != "http-basic-auth-cancel"
                        username = HTTP_AUTH_USERNAME if accepted else ""
                        password = HTTP_AUTH_PASSWORD if accepted else ""
                        send_message(
                            conn,
                            38,
                            http_auth_reply_payload(
                                request_tab_id,
                                request_id,
                                accepted,
                                username,
                                password,
                            ),
                        )
                        if not accepted:
                            http_auth_cancel_sent_at = time.time()
                        reply_received_at = time.time()
                        evidence = {
                            "request_time": time.time() - start,
                            "reply_received_at": reply_received_at,
                            "tab_id": request_tab_id,
                            "request_id": request_id,
                            "url": url_value,
                            "auth_scheme": auth_scheme,
                            "challenger": challenger,
                            "realm": realm,
                            "is_proxy": is_proxy,
                            "first_auth_attempt": first_auth_attempt,
                            "accepted": accepted,
                            "reply_request_id": request_id,
                            "reply_time": time.time() - start,
                        }
                        http_auth.append(evidence)
                        messages.write(
                            "http_auth "
                            f"request_id={request_id} scheme={auth_scheme} "
                            f"realm={realm} accepted={accepted}\n"
                        )
                        messages.flush()
                    elif top == 39:
                        fields = parse_message_fields(body)
                        evidence = {
                            "received_time": time.time() - start,
                            "tab_id": int(fields.get(1, 0) or 0),
                            "termination_status": str(fields.get(2, "")),
                            "termination_status_code": int(fields.get(3, 0) or 0),
                            "url": str(fields.get(4, "")),
                            "can_reload": bool(fields.get(5, False)),
                        }
                        renderer_crashes.append(evidence)
                        messages.write(
                            "renderer_crashed "
                            f"status={evidence['termination_status']} "
                            f"code={evidence['termination_status_code']} "
                            f"url={evidence['url']} "
                            f"can_reload={evidence['can_reload']}\n"
                        )
                        messages.flush()
                except socket.timeout:
                    pass
    finally:
        if webauthn_devtools_process is not None:
            try:
                webauthn_devtools_process.terminate()
                webauthn_devtools_process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                webauthn_devtools_process.kill()
                webauthn_devtools_process.wait(timeout=2)
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
        stdout.close()
        stderr.close()
        listener.close()
        try:
            socket_path.unlink()
        except FileNotFoundError:
            pass

    stderr_text = read_text(stderr_path)
    stdout_text = read_text(stdout_path)
    log_scan = scan_logs(
        stderr_text + "\n" + stdout_text,
        expect_renderer_crash=probe.name == "renderer-crash-recovery",
    )
    reports = state_reports_for_probe(run_dir, probe.name)
    auth_events = state.auth_events_for(probe.name)
    non_timeout_reports = [
        candidate for candidate in reports if candidate.get("status") != "page_timeout"
    ]
    report = (non_timeout_reports or reports)[-1] if reports else None
    proc_exit = proc.returncode
    classification = classify_probe(report, log_scan, proc_exit)
    download_result = None
    if expected_download:
        download_result = verify_download(
            download_dir,
            expected_download[0],
            expected_download[1],
            time.time() + 0.1,
        )
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and download_result["status"] == "completed"
        ):
            classification = "download_completed"
        elif classification == "exercised":
            classification = "download_failed"
    javascript_dialog_result = verify_javascript_dialog_probe(
        probe.name,
        report,
        javascript_dialogs,
        activation_observed_at is not None,
    )
    if javascript_dialog_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and javascript_dialog_result["status"] == "completed"
        ):
            classification = "dialog_completed"
        elif classification == "exercised":
            classification = "dialog_failed"
    default_deny_result = verify_default_deny_probe(probe.name, report)
    if default_deny_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and default_deny_result["status"] == "completed"
        ):
            classification = "default_denied"
        elif classification in ("exercised", "reported"):
            classification = "default_deny_failed"
    file_system_access_result = verify_file_system_access_probe(
        probe.name,
        report,
        activation_ready_at,
        activation_sent,
        activation_observed_at,
        picker_call_started_at,
    )
    if file_system_access_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and file_system_access_result["status"] == "completed"
        ):
            classification = "file_system_access_denied"
        elif classification in (
            "exercised",
            "reported",
            "blocked_user_activation",
            "page_timeout",
        ):
            classification = "file_system_access_failed"
    webauthn_result = verify_webauthn_probe(
        probe.name,
        report,
        activation_ready_at,
        activation_sent,
        activation_observed_at,
        webauthn_devtools_setup,
    )
    if webauthn_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and webauthn_result["status"] == "completed"
        ):
            classification = "webauthn_virtual_authenticator_completed"
        elif classification in (
            "exercised",
            "reported",
            "blocked_user_activation",
            "blocked_needs_virtual_authenticator",
            "page_timeout",
            "probe_timeout",
        ):
            classification = "webauthn_virtual_authenticator_failed"
    page_zoom_result = verify_page_zoom_probe(
        probe.name,
        reports,
        page_zoom_events,
    )
    if page_zoom_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and page_zoom_result["status"] == "completed"
        ):
            classification = "page_zoom_completed"
        elif classification in ("exercised", "reported"):
            classification = "page_zoom_failed"
    console_capture_result = verify_console_capture_probe(
        probe.name,
        tab_id,
        report,
        console_messages,
    )
    if console_capture_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and console_capture_result["status"] == "completed"
        ):
            classification = "console_capture_completed"
        elif classification in ("exercised", "reported"):
            classification = "console_capture_failed"
    http_auth_result = verify_http_auth_probe(
        probe.name,
        tab_id,
        report,
        reports,
        http_auth,
        auth_events,
        stderr_text + "\n" + stdout_text + "\n" + read_text(messages_path),
    )
    if http_auth_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and http_auth_result["status"] == "completed"
        ):
            classification = (
                "http_auth_cancelled"
                if probe.name == "http-basic-auth-cancel"
                else "http_auth_completed"
            )
        elif classification in ("exercised", "reported"):
            classification = "http_auth_failed"
    renderer_crash_result = verify_renderer_crash_probe(
        probe.name,
        tab_id,
        reports,
        loading_states,
        renderer_crashes,
        crash_navigation_sent,
        post_crash_navigation_sent,
        socket_disconnect,
        proc_exit,
    )
    if renderer_crash_result:
        if (
            classification
            not in (
                "missing_binder",
                "bad_mojo",
                "renderer_or_browser_crash",
                "process_exit",
            )
            and renderer_crash_result["status"] == "completed"
        ):
            classification = "renderer_crash_recovered"
        elif classification in ("exercised", "reported", "no_report", "page_timeout"):
            classification = "renderer_crash_unrecovered"
    result = {
        "probe": probe.name,
        "feature": probe.feature,
        "url": url,
        "requires_user_activation": probe.requires_user_activation,
        "tab_id": tab_id,
        "sent_create_tab": sent_create,
        "socket_disconnect": socket_disconnect,
        "process_exit_code": proc_exit,
        "report": report,
        "page_reported": report is not None,
        "bad_mojo": log_scan["bad_mojo"],
        "crashed": log_scan["crashed"],
        "missing_interfaces": log_scan["missing_interfaces"],
        "empty_interfaces": log_scan["empty_interfaces"],
        "classification": classification,
        "download": download_result,
        "javascript_dialogs": javascript_dialogs,
        "javascript_dialog_result": javascript_dialog_result,
        "default_deny_result": default_deny_result,
        "file_system_access_result": file_system_access_result,
        "webauthn_result": webauthn_result,
        "webauthn_devtools_setup": webauthn_devtools_setup,
        "page_zoom_result": page_zoom_result,
        "console_messages": console_messages,
        "console_capture_result": console_capture_result,
        "http_auth": http_auth,
        "http_auth_server_events": auth_events,
        "http_auth_result": http_auth_result,
        "loading_states": loading_states,
        "renderer_crashes": renderer_crashes,
        "expected_crash_lines": log_scan["expected_crash_lines"],
        "unexpected_crash_lines": log_scan["unexpected_crash_lines"],
        "renderer_crash_result": renderer_crash_result,
        "beforeunload_activation": (
            {
                "ready": activation_ready_at is not None,
                "input_sent": activation_sent,
                "activation_observed": activation_observed_at is not None,
                "navigation_sent": navigation_sent,
            }
            if probe.name.startswith("javascript-beforeunload-")
            else None
        ),
        "file_system_access_activation": (
            {
                "ready": activation_ready_at is not None,
                "activation_sent": activation_sent,
                "activation_observed": activation_observed_at is not None,
                "picker_call_started_after_activation": (
                    picker_call_started_at is not None
                ),
            }
            if probe.name == "file-system-access"
            else None
        ),
        "webauthn_activation": (
            {
                "ready": activation_ready_at is not None,
                "activation_sent": activation_sent,
                "activation_observed": activation_observed_at is not None,
            }
            if probe.name == "webauthn-create"
            else None
        ),
        "log_dir": str(probe_dir),
    }
    write_json(probe_dir / "probe-result.json", result)
    append_file(run_dir / "roamium.stdout", f"\n===== {probe.name} =====\n" + stdout_text)
    append_file(run_dir / "roamium.stderr", f"\n===== {probe.name} =====\n" + stderr_text)
    append_file(run_dir / "messages.log", f"\n===== {probe.name} =====\n" + read_text(messages_path))
    return result


def state_reports_for_probe(run_dir: pathlib.Path, probe: str) -> list[dict[str, Any]]:
    reports_path = run_dir / "reports.jsonl"
    reports: list[dict[str, Any]] = []
    if not reports_path.exists():
        return reports
    for line in reports_path.read_text(encoding="utf-8").splitlines():
        try:
            report = json.loads(line)
        except json.JSONDecodeError:
            continue
        if report.get("probe") == probe:
            reports.append(report)
    return reports


def append_file(path: pathlib.Path, text: str) -> None:
    with path.open("a", encoding="utf-8") as out:
        out.write(text)


def write_binder_errors(path: pathlib.Path, results: list[dict[str, Any]]) -> None:
    lines = ["probe\ttype\tinterface\n"]
    for result in results:
        for interface in result.get("missing_interfaces", []):
            lines.append(f"{result['probe']}\tmissing\t{interface}\n")
        for interface in result.get("empty_interfaces", []):
            lines.append(f"{result['probe']}\tempty\t{interface}\n")
    path.write_text("".join(lines), encoding="utf-8")


def actionable_empty_interfaces(result: dict[str, Any]) -> list[str]:
    if result.get("classification") != "empty_binder":
        return []
    return [
        interface
        for interface in result.get("empty_interfaces", [])
        if interface != "blink.mojom.LCPCriticalPathPredictorHost"
    ]


def write_coverage_map(path: pathlib.Path, results: list[dict[str, Any]]) -> None:
    lines = [
        "# Issue 799 Browser API Probe Coverage",
        "",
        "| Probe | Feature | Classification | JS status | Missing interface | Next action |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for result in results:
        report = result.get("report") or {}
        missing = ", ".join(result.get("missing_interfaces") or []) or "-"
        if result.get("empty_interfaces"):
            missing = "empty: " + ", ".join(result.get("empty_interfaces") or [])
        action_empty = ", ".join(actionable_empty_interfaces(result))
        classification = result["classification"]
        if classification == "missing_binder":
            next_action = "Design a narrow binder/stub experiment."
        elif classification == "empty_binder":
            next_action = (
                f"Replace empty binder with narrow TermSurf behavior or explicit denial: {action_empty}."
                if action_empty
                else "Review empty binder; no actionable non-ambient interface extracted."
            )
        elif classification == "blocked_user_activation":
            next_action = "Needs synthetic activation coverage before claiming binder safety."
        elif classification == "blocked_needs_virtual_authenticator":
            next_action = "Needs DevTools virtual authenticator coverage before claiming WebAuthn safety."
        elif classification == "default_denied":
            next_action = "Permission API request/query path returned TermSurf default denial."
        elif classification == "default_deny_failed":
            next_action = "Inspect TermSurf permission manager routing and denial evidence."
        elif classification == "file_system_access_denied":
            next_action = "File System Access reached post-activation TermSurf denial."
        elif classification == "file_system_access_failed":
            next_action = "Inspect activation routing and file picker denial evidence."
        elif classification == "webauthn_virtual_authenticator_completed":
            next_action = "WebAuthn completed against a DevTools virtual authenticator."
        elif classification == "webauthn_virtual_authenticator_failed":
            next_action = "Inspect DevTools virtual-authenticator setup and credential evidence."
        elif classification == "browser_service_unavailable":
            next_action = "Browser service unavailable; inspect logs and reference binders."
        elif classification == "exercised":
            next_action = "No action from this probe; expand coverage if needed."
        elif classification == "download_completed":
            next_action = "Generic download completed with verified file bytes."
        elif classification == "download_failed":
            next_action = "Inspect download target selection and file evidence."
        elif classification == "dialog_completed":
            next_action = "JavaScript dialog completed through request/reply protocol."
        elif classification == "dialog_failed":
            next_action = "Inspect JavaScript dialog request/reply evidence."
        elif classification == "page_zoom_completed":
            next_action = "Page zoom shortcuts completed with verified browser zoom metrics."
        elif classification == "page_zoom_failed":
            next_action = "Inspect page zoom shortcut routing and metric snapshots."
        elif classification == "console_capture_completed":
            next_action = "Console messages completed through protocol capture."
        elif classification == "console_capture_failed":
            next_action = "Inspect console protocol messages and source/level evidence."
        elif classification == "http_auth_completed":
            next_action = "HTTP Basic Auth completed through request/reply protocol."
        elif classification == "http_auth_cancelled":
            next_action = "HTTP Basic Auth cancellation completed and tab stayed usable."
        elif classification == "http_auth_failed":
            next_action = "Inspect HTTP auth request/reply and server sequence evidence."
        elif classification == "renderer_crash_recovered":
            next_action = "Renderer crash was reported and same-tab recovery completed."
        elif classification == "renderer_crash_unrecovered":
            next_action = "Inspect crash event and post-crash navigation evidence."
        elif classification == "unsupported":
            next_action = "No runtime surface exposed in this build."
        else:
            next_action = "Investigate harness or browser behavior."
        lines.append(
            "| {probe} | {feature} | {classification} | {status} | {missing} | {next_action} |".format(
                probe=result["probe"],
                feature=result["feature"],
                classification=classification,
                status=report.get("status", "-"),
                missing=missing,
                next_action=next_action,
            )
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_reference_coverage_map(path: pathlib.Path, results: list[dict[str, Any]]) -> None:
    by_name = {result["probe"]: result for result in results}
    lines = [
        "# Issue 799 Reference Coverage Map",
        "",
        "| JS API / feature | Expected browser-side surface | Reference evidence | TermSurf evidence | Runtime probe status | Next action |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for probe in PROBES:
        result = by_name.get(probe.name, {})
        classification = result.get("classification", "not_run")
        missing = ", ".join(result.get("missing_interfaces") or [])
        empty = ", ".join(actionable_empty_interfaces(result))
        if missing:
            next_action = f"Fix missing interface: {missing}."
        elif empty and classification == "empty_binder":
            next_action = f"Replace empty binder or add explicit denial for: {empty}."
        elif classification == "blocked_user_activation":
            next_action = "Add contained user-activation probe before claiming coverage."
        elif classification == "blocked_needs_virtual_authenticator":
            next_action = "Add contained DevTools virtual-authenticator coverage before claiming coverage."
        elif classification == "default_denied":
            next_action = "No immediate action; verified TermSurf default-deny permission policy."
        elif classification == "default_deny_failed":
            next_action = "Inspect TermSurf permission manager routing and denial evidence."
        elif classification == "file_system_access_denied":
            next_action = "No immediate action; verified post-activation file picker denial."
        elif classification == "file_system_access_failed":
            next_action = "Inspect activation routing and file picker denial evidence."
        elif classification == "webauthn_virtual_authenticator_completed":
            next_action = "No immediate action; verified contained virtual-authenticator coverage."
        elif classification == "webauthn_virtual_authenticator_failed":
            next_action = "Inspect DevTools virtual-authenticator setup and credential evidence."
        elif classification == "download_completed":
            next_action = "No immediate action; verified generic download completion."
        elif classification == "page_zoom_completed":
            next_action = "No immediate action; verified page zoom shortcuts."
        elif classification == "page_zoom_failed":
            next_action = "Inspect page zoom shortcut routing and metric snapshots."
        elif classification == "console_capture_completed":
            next_action = "No immediate action; verified protocol console capture."
        elif classification == "console_capture_failed":
            next_action = "Inspect console protocol messages and source/level evidence."
        elif classification == "http_auth_completed":
            next_action = "No immediate action; verified HTTP Basic Auth success."
        elif classification == "http_auth_cancelled":
            next_action = "No immediate action; verified HTTP Basic Auth cancellation."
        elif classification == "http_auth_failed":
            next_action = "Inspect HTTP auth request/reply and server sequence evidence."
        elif classification == "renderer_crash_recovered":
            next_action = "No immediate action; verified renderer crash recovery."
        elif classification == "renderer_crash_unrecovered":
            next_action = "Inspect crash event and same-tab recovery evidence."
        elif classification in ("exercised", "unsupported"):
            next_action = "No immediate binder fix from this probe."
        else:
            next_action = "Investigate harness/runtime result."
        lines.append(
            "| {feature} | {surface} | {reference} | {termsurf} | {status} | {next_action} |".format(
                feature=probe.feature,
                surface=probe.expected_surface,
                reference=probe.reference_evidence,
                termsurf=probe.termsurf_evidence,
                status=classification,
                next_action=next_action,
            )
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def chromium_branch() -> str | None:
    try:
        proc = subprocess.run(
            ["git", "rev-parse", "--abbrev-ref", "HEAD"],
            cwd=ROOT / "chromium/src",
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None
    return proc.stdout.strip()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--roamium", type=pathlib.Path, default=DEFAULT_ROAMIUM)
    parser.add_argument("--log-dir", type=pathlib.Path)
    parser.add_argument("--seconds", type=float, default=8.0)
    parser.add_argument("--width", type=int, default=1200)
    parser.add_argument("--height", type=int, default=900)
    parser.add_argument(
        "--probe",
        action="append",
        choices=[probe.name for probe in PROBES],
        help="Run only the named probe. May be passed more than once.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    roamium = args.roamium.resolve()
    if not roamium.exists():
        raise SystemExit(f"missing Roamium binary: {roamium}")

    run_dir = (args.log_dir or DEFAULT_LOG_ROOT / timestamp()).resolve()
    run_dir.mkdir(parents=True, exist_ok=True)
    download_dir = run_dir / "downloads"
    download_dir.mkdir(parents=True, exist_ok=True)
    state = ProbeState(run_dir)
    server = start_server(state)
    host, port = server.server_address
    base_url = f"http://localhost:{port}"
    selected = [probe for probe in PROBES if not args.probe or probe.name in args.probe]
    start = dt.datetime.now(dt.timezone.utc)

    run_info: dict[str, Any] = {
        "command": sys.argv,
        "roamium": str(roamium),
        "chromium_branch": chromium_branch(),
        "fixture_base_url": base_url,
        "download_dir": str(download_dir),
        "started_at": start.isoformat(),
        "probe_count": len(selected),
        "logging": {
            "flags": ["--enable-logging=stderr"],
            "stderr": str(run_dir / "roamium.stderr"),
            "stdout": str(run_dir / "roamium.stdout"),
            "bad_mojo_patterns": BAD_MOJO_PATTERNS,
        },
    }
    write_json(run_dir / "run.json", run_info)

    try:
        results = [
            run_probe(
                probe,
                base_url,
                state,
                run_dir,
                roamium,
                download_dir,
                args.seconds,
                args.width,
                args.height,
            )
            for probe in selected
        ]
    finally:
        server.shutdown()

    any_missing = any(
        result["missing_interfaces"] or result["classification"] == "empty_binder"
        for result in results
    )
    any_crash = any(result["crashed"] for result in results)
    run_status = "missing_binder" if any_missing else "crash" if any_crash else "completed"
    run_info.update(
        {
            "finished_at": dt.datetime.now(dt.timezone.utc).isoformat(),
            "status": run_status,
            "classifications": {
                result["probe"]: result["classification"] for result in results
            },
            "missing_interfaces": sorted(
                {
                    interface
                    for result in results
                    for interface in result.get("missing_interfaces", [])
                }
            ),
            "empty_interfaces": sorted(
                {
                    interface
                    for result in results
                    for interface in result.get("empty_interfaces", [])
                }
            ),
        }
    )
    write_json(run_dir / "run.json", run_info)
    write_json(run_dir / "probe-results.json", results)
    write_binder_errors(run_dir / "binder-errors.tsv", results)
    write_coverage_map(run_dir / "coverage-map.md", results)
    write_reference_coverage_map(run_dir / "reference-coverage-map.md", results)

    print(run_dir)
    print(json.dumps(run_info, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
