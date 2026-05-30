#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";

function parseArgs(argv) {
  const args = { timeoutSeconds: 30, settleSeconds: 8, actionSettleMs: 800 };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith("--")) {
      throw new Error(`unexpected positional argument: ${arg}`);
    }
    const [rawKey, inlineValue] = arg.slice(2).split("=", 2);
    const key = rawKey.replace(/-([a-z])/g, (_, ch) => ch.toUpperCase());
    const value = inlineValue ?? argv[++i];
    if (value === undefined) {
      throw new Error(`missing value for ${arg}`);
    }
    args[key] = value;
  }
  for (const key of ["devtoolsPort", "urlContains", "outDir"]) {
    if (!args[key]) {
      throw new Error(
        `missing --${key.replace(/[A-Z]/g, (ch) => `-${ch.toLowerCase()}`)}`,
      );
    }
  }
  args.devtoolsPort = Number(args.devtoolsPort);
  args.timeoutSeconds = Number(args.timeoutSeconds);
  args.settleSeconds = Number(args.settleSeconds);
  args.actionSettleMs = Number(args.actionSettleMs);
  args.outDir = path.resolve(args.outDir);
  return args;
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

async function fetchJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`GET ${url} failed: HTTP ${response.status}`);
  }
  return await response.json();
}

async function pollTarget(args, summary) {
  const deadline = Date.now() + args.timeoutSeconds * 1000;
  const listUrl = `http://127.0.0.1:${args.devtoolsPort}/json/list`;
  let lastTargets = [];
  while (Date.now() < deadline) {
    lastTargets = await fetchJson(listUrl);
    const target = lastTargets.find(
      (item) =>
        item.type === "page" &&
        typeof item.url === "string" &&
        item.url.includes(args.urlContains) &&
        item.webSocketDebuggerUrl,
    );
    if (target) {
      return target;
    }
    await sleep(250);
  }
  summary.availableTargets = lastTargets.map((item) => ({
    id: item.id,
    type: item.type,
    url: item.url,
    title: item.title,
  }));
  throw new Error(
    `no page target contained ${JSON.stringify(args.urlContains)}`,
  );
}

function connectDevTools(wsUrl) {
  const socket = new WebSocket(wsUrl);
  let nextId = 1;
  const pending = new Map();
  const events = [];

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) {
        reject(
          new Error(
            `${message.error.message || "DevTools error"} (${message.error.code})`,
          ),
        );
      } else {
        resolve(message.result || {});
      }
      return;
    }
    if (message.method) {
      events.push(message);
    }
  });

  const open = new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  function send(method, params = {}, sessionId = undefined) {
    const id = nextId;
    nextId += 1;
    const promise = new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
    });
    const message = { id, method, params };
    if (sessionId) {
      message.sessionId = sessionId;
    }
    socket.send(JSON.stringify(message));
    return promise;
  }

  return { socket, open, send, events };
}

async function safeSend(client, method, params = {}, sessionId = undefined) {
  try {
    return { ok: true, result: await client.send(method, params, sessionId) };
  } catch (error) {
    return { ok: false, error: String(error.message || error) };
  }
}

async function evaluate(client, expression, sessionId = undefined) {
  const result = await safeSend(
    client,
    "Runtime.evaluate",
    { expression, awaitPromise: true, returnByValue: true },
    sessionId,
  );
  if (!result.ok) {
    return { ok: false, error: result.error };
  }
  if (result.result.exceptionDetails) {
    return { ok: false, error: JSON.stringify(result.result.exceptionDetails) };
  }
  return { ok: true, value: result.result.result?.value ?? null };
}

async function captureScreenshot(client, args, relativePath) {
  const result = await client.send("Page.captureScreenshot", {
    format: "png",
    fromSurface: true,
  });
  const png = Buffer.from(result.data || "", "base64");
  const filePath = path.join(args.outDir, relativePath);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, png);
  return {
    relativePath,
    bytes: png.length,
    sha256: crypto.createHash("sha256").update(png).digest("hex"),
  };
}

const SETUP_SOURCE = `(() => {
  const trace = [];
  let activeActionId = "setup";
  let sequence = 0;
  const safeArg = (arg) => {
    if (arg === null || arg === undefined) return arg;
    if (["string", "number", "boolean"].includes(typeof arg)) return arg;
    if (arg instanceof Event) return {eventType: arg.type, bubbles: arg.bubbles, composed: arg.composed};
    return Object.prototype.toString.call(arg);
  };
  const log = (type, data = {}) => {
    trace.push({seq: ++sequence, actionId: activeActionId, type, time: performance.now(), ...data});
  };
  const token = (el) => el ? [
    el.getAttribute?.("aria-label") || "",
    el.getAttribute?.("title") || "",
    el.id || "",
    String(el.className || ""),
    el.getAttribute?.("role") || "",
    el.innerText || "",
    el.value || "",
  ].join(" ").toLowerCase() : "";
  const rectOf = (el) => {
    const rect = el?.getBoundingClientRect?.();
    return {
      tag: el?.tagName || "",
      id: el?.id || "",
      token: token(el),
      x: rect?.x || 0,
      y: rect?.y || 0,
      width: rect?.width || 0,
      height: rect?.height || 0,
      disabled: !!el?.disabled || el?.getAttribute?.("aria-disabled") === "true",
      hidden: !!el?.hidden || (el ? getComputedStyle(el).display === "none" || getComputedStyle(el).visibility === "hidden" : true),
    };
  };
  const viewer = document.querySelector("pdf-viewer#viewer") || document.querySelector("pdf-viewer");
  const viewerRoot = viewer?.shadowRoot || null;
  const toolbar = viewerRoot?.querySelector("viewer-toolbar#toolbar") || viewerRoot?.querySelector("viewer-toolbar") || null;
  const toolbarRoot = toolbar?.shadowRoot || null;
  const allControls = [...(toolbarRoot?.querySelectorAll("button,input,cr-icon-button,*[role=button]") || [])];
  const controls = {
    zoomIn: allControls.find((el) => token(el).includes("zoomin") || token(el).includes("zoom in")) || null,
    zoomOut: allControls.find((el) => token(el).includes("zoomout") || token(el).includes("zoom out")) || null,
    rotate: toolbarRoot?.querySelector("#rotate") || allControls.find((el) => token(el).includes("rotate")) || null,
    fit: toolbarRoot?.querySelector("#fit") || allControls.find((el) => token(el).includes("fit")) || null,
  };
  const setup = {
    viewerFound: !!viewer,
    toolbarFound: !!toolbar,
    toolbarRootFound: !!toolbarRoot,
    controls: Object.fromEntries(Object.entries(controls).map(([name, el]) => [name, rectOf(el)])),
    listenerInstallations: [],
    wrapResults: [],
  };
  const eventTypes = ["click", "zoom-in", "zoom-out", "rotate-left", "fit-to-changed"];
  const nodes = [
    ["control.zoomIn", controls.zoomIn],
    ["control.zoomOut", controls.zoomOut],
    ["control.rotate", controls.rotate],
    ["control.fit", controls.fit],
    ["toolbarShadowRoot", toolbarRoot],
    ["toolbar", toolbar],
    ["viewerShadowRoot", viewerRoot],
    ["viewer", viewer],
    ["document", document],
  ];
  for (const [label, node] of nodes) {
    if (!node?.addEventListener) continue;
    for (const eventType of eventTypes) {
      for (const capture of [true, false]) {
        node.addEventListener(eventType, (event) => {
          log("event", {
            listener: label,
            eventType,
            phase: capture ? "capture" : "bubble",
            target: rectOf(event.target),
            currentTarget: label,
            bubbles: event.bubbles,
            composed: event.composed,
          });
        }, {capture});
        setup.listenerInstallations.push({label, eventType, phase: capture ? "capture" : "bubble"});
      }
    }
  }
  const findDescriptor = (obj, name) => {
    let current = obj;
    let depth = 0;
    while (current) {
      const descriptor = Object.getOwnPropertyDescriptor(current, name);
      if (descriptor) {
        return {owner: current, descriptor, depth};
      }
      current = Object.getPrototypeOf(current);
      depth += 1;
    }
    return null;
  };
  const wrapMethod = (obj, name, label) => {
    const result = {label, name, found: false, wrapped: false};
    if (!obj) {
      result.wrap_failed = "owner-missing";
      setup.wrapResults.push(result);
      return;
    }
    const found = findDescriptor(obj, name);
    if (!found) {
      result.wrap_failed = "not-found";
      setup.wrapResults.push(result);
      return;
    }
    const {owner, descriptor, depth} = found;
    result.found = true;
    result.prototypeDepth = depth;
    result.descriptor = {
      hasValue: "value" in descriptor,
      hasGet: !!descriptor.get,
      hasSet: !!descriptor.set,
      writable: !!descriptor.writable,
      configurable: !!descriptor.configurable,
    };
    const original = descriptor.value;
    if (typeof original !== "function") {
      result.wrap_failed = "not-function";
      setup.wrapResults.push(result);
      return;
    }
    if (!descriptor.writable && !descriptor.configurable) {
      result.wrap_failed = "not-writable";
      setup.wrapResults.push(result);
      return;
    }
    try {
      Object.defineProperty(owner, name, {
        ...descriptor,
        value: function(...args) {
          log("method-enter", {label, name, args: args.map(safeArg)});
          try {
            const returned = original.apply(this, args);
            log("method-return", {label, name, returned: safeArg(returned)});
            return returned;
          } catch (error) {
            log("method-throw", {label, name, error: String(error?.stack || error)});
            throw error;
          }
        },
      });
      result.wrapped = true;
    } catch (error) {
      result.wrap_failed = String(error?.message || error);
    }
    setup.wrapResults.push(result);
  };
  wrapMethod(toolbar, "onZoomInClick_", "toolbar.onZoomInClick_");
  wrapMethod(toolbar, "onZoomOutClick_", "toolbar.onZoomOutClick_");
  wrapMethod(toolbar, "onRotateClick_", "toolbar.onRotateClick_");
  wrapMethod(toolbar, "onFitToButtonClick_", "toolbar.onFitToButtonClick_");
  wrapMethod(viewer, "onZoomIn", "viewer.onZoomIn");
  wrapMethod(viewer, "onZoomOut", "viewer.onZoomOut");
  wrapMethod(viewer, "onRotateLeft_", "viewer.onRotateLeft_");
  wrapMethod(viewer, "onFitToChanged", "viewer.onFitToChanged");
  wrapMethod(viewer?.viewport_, "zoomIn", "viewer.viewport_.zoomIn");
  wrapMethod(viewer?.viewport_, "zoomOut", "viewer.viewport_.zoomOut");
  wrapMethod(viewer?.viewport_, "setFittingType", "viewer.viewport_.setFittingType");
  wrapMethod(viewer?.currentController, "rotateCounterclockwise", "viewer.currentController.rotateCounterclockwise");
  window.__termsurfPdfToolbarTrace = trace;
  window.__termsurfPdfToolbarSetup = setup;
  window.__termsurfPdfToolbarSetAction = (actionId) => {
    activeActionId = actionId;
    log("action-start", {actionId});
    return {actionId, traceLength: trace.length};
  };
  window.__termsurfPdfToolbarSnapshot = () => ({setup, trace, actionId: activeActionId});
  log("setup-complete", setup);
  return setup;
})()`;

const STATE_SOURCE = `(() => {
  const viewer = document.querySelector("pdf-viewer#viewer") || document.querySelector("pdf-viewer");
  const viewerRoot = viewer?.shadowRoot || null;
  const toolbar = viewerRoot?.querySelector("viewer-toolbar#toolbar") || viewerRoot?.querySelector("viewer-toolbar") || null;
  const toolbarRoot = toolbar?.shadowRoot || null;
  const zoomInput = toolbarRoot?.querySelector('input[aria-label*="zoom"], input');
  const fit = toolbarRoot?.querySelector("#fit");
  const rotate = toolbarRoot?.querySelector("#rotate");
  const pageSelector = toolbarRoot?.querySelector("#pageSelector");
  const primitiveValue = (value) => {
    if (value === null || ["string", "number", "boolean"].includes(typeof value)) {
      return value;
    }
    if (Array.isArray(value)) {
      return {type: "array", length: value.length};
    }
    return undefined;
  };
  const primitiveProps = (obj) => {
    const out = {};
    if (!obj) return out;
    let current = obj;
    let depth = 0;
    const seen = new Set();
    while (current && depth < 4) {
      for (const name of Object.getOwnPropertyNames(current).sort()) {
        if (seen.has(name) || name === "constructor") continue;
        seen.add(name);
        try {
          const descriptor = Object.getOwnPropertyDescriptor(current, name);
          if (!descriptor) continue;
          if ("value" in descriptor) {
            const value = primitiveValue(obj[name]);
            if (value !== undefined) {
              out[name] = {depth, value};
            }
          } else if (descriptor.get) {
            const value = primitiveValue(obj[name]);
            if (value !== undefined) {
              out[name] = {depth, value, accessor: true};
            }
          }
        } catch (error) {
          out[name] = {depth, error: String(error)};
        }
      }
      current = Object.getPrototypeOf(current);
      depth += 1;
    }
    return out;
  };
  const rect = (el) => {
    const box = el?.getBoundingClientRect?.();
    return box ? {x: box.x, y: box.y, width: box.width, height: box.height} : null;
  };
  const token = (el) => el ? [
    el.tagName || "",
    el.id || "",
    String(el.className || ""),
    el.getAttribute?.("part") || "",
    el.getAttribute?.("aria-label") || "",
    el.getAttribute?.("role") || "",
  ].join(" ").trim() : "";
  const styleSummary = (el) => {
    if (!el) return null;
    const style = getComputedStyle(el);
    return {
      display: style.display,
      visibility: style.visibility,
      overflow: style.overflow,
      overflowX: style.overflowX,
      overflowY: style.overflowY,
      transform: style.transform,
    };
  };
  const collectInteresting = () => {
    const roots = [{name: "document", root: document}];
    if (viewerRoot) roots.push({name: "viewerShadowRoot", root: viewerRoot});
    if (toolbarRoot) roots.push({name: "toolbarShadowRoot", root: toolbarRoot});
    const selectors = [
      "pdf-viewer",
      "viewer-toolbar",
      "#scroller",
      "#sizer",
      "#viewerContainer",
      "#viewer",
      "#plugin",
      "embed",
      "iframe",
      "canvas",
      ".page",
      ".page-container",
      "[data-page-number]",
      "[style*='transform']",
      "[style*='width']",
      "[style*='height']",
    ].join(",");
    const seen = new Set();
    const nodes = [];
    for (const {name, root} of roots) {
      for (const el of root.querySelectorAll?.(selectors) || []) {
        if (seen.has(el)) continue;
        seen.add(el);
        const box = rect(el);
        nodes.push({
          root: name,
          token: token(el),
          rect: box,
          scrollLeft: "scrollLeft" in el ? el.scrollLeft : null,
          scrollTop: "scrollTop" in el ? el.scrollTop : null,
          scrollWidth: "scrollWidth" in el ? el.scrollWidth : null,
          scrollHeight: "scrollHeight" in el ? el.scrollHeight : null,
          style: styleSummary(el),
        });
      }
    }
    return nodes;
  };
  return {
    url: location.href,
    title: document.title,
    zoomText: zoomInput?.value || "",
    pageText: pageSelector?.value || "",
    viewer: rect(viewer),
    fit: rect(fit),
    rotate: rect(rotate),
    viewport: {innerWidth, innerHeight, devicePixelRatio},
    documentScroll: {
      scrollX,
      scrollY,
      bodyScrollTop: document.body?.scrollTop || 0,
      bodyScrollHeight: document.body?.scrollHeight || 0,
      documentScrollTop: document.documentElement?.scrollTop || 0,
      documentScrollHeight: document.documentElement?.scrollHeight || 0,
    },
    interestingNodes: collectInteresting(),
    viewerProps: primitiveProps(viewer),
    viewportProps: primitiveProps(viewer?.viewport_),
    controllerProps: primitiveProps(viewer?.currentController),
  };
})()`;

const RESOURCES_PRIVATE_SOURCE = `(() => new Promise((resolve) => {
  const viewer = document.querySelector("pdf-viewer#viewer") || document.querySelector("pdf-viewer");
  const getPresetSummary = () => {
    const factors = viewer?.viewport_?.presetZoomFactors_ || viewer?.viewport_?.presetZoomFactors || null;
    return {
      isArray: Array.isArray(factors),
      length: Array.isArray(factors) ? factors.length : null,
      values: Array.isArray(factors) ? factors.slice(0, 20) : null,
      fittingType: viewer?.viewport_?.fittingType_ || null,
      viewportZoom: viewer?.viewportZoom_ ?? null,
    };
  };
  const out = {
    chromeExists: typeof chrome !== "undefined",
    resourcesPrivateExists: typeof chrome !== "undefined" && !!chrome.resourcesPrivate,
    getStringsExists: typeof chrome !== "undefined" && typeof chrome.resourcesPrivate?.getStrings === "function",
    componentPdfValue: typeof chrome !== "undefined" ? chrome.resourcesPrivate?.Component?.PDF ?? null : null,
    beforeViewportPreset: getPresetSummary(),
    callbackFired: false,
    lastErrorBefore: typeof chrome !== "undefined" ? chrome.runtime?.lastError?.message || null : null,
  };
  if (!out.getStringsExists) {
    out.afterViewportPreset = getPresetSummary();
    resolve(out);
    return;
  }
  let settled = false;
  const finish = () => {
    if (settled) return;
    settled = true;
    out.afterViewportPreset = getPresetSummary();
    resolve(out);
  };
  setTimeout(() => {
    out.timeout = true;
    out.lastErrorAfterTimeout = chrome.runtime?.lastError?.message || null;
    finish();
  }, 3000);
  try {
    chrome.resourcesPrivate.getStrings(chrome.resourcesPrivate.Component.PDF, (strings) => {
      out.callbackFired = true;
      out.callbackType = Object.prototype.toString.call(strings);
      out.lastErrorAfterCallback = chrome.runtime?.lastError?.message || null;
      out.keys = strings ? Object.keys(strings).sort() : [];
      out.hasPresetZoomFactors = !!strings && Object.prototype.hasOwnProperty.call(strings, "presetZoomFactors");
      out.presetZoomFactorsRaw = strings?.presetZoomFactors ?? null;
      try {
        const parsed = JSON.parse(strings?.presetZoomFactors || "null");
        out.presetZoomFactorsParsedIsArray = Array.isArray(parsed);
        out.presetZoomFactorsParsedLength = Array.isArray(parsed) ? parsed.length : null;
        out.presetZoomFactorsParsedValues = Array.isArray(parsed) ? parsed.slice(0, 20) : null;
      } catch (error) {
        out.presetZoomFactorsParseError = String(error?.message || error);
      }
      finish();
    });
  } catch (error) {
    out.threw = String(error?.stack || error);
    finish();
  }
}))()`;

async function setAction(client, sessionId, actionId) {
  return await evaluate(
    client,
    `window.__termsurfPdfToolbarSetAction(${JSON.stringify(actionId)})`,
    sessionId,
  );
}

async function snapshotTrace(client, sessionId) {
  return await evaluate(
    client,
    "window.__termsurfPdfToolbarSnapshot()",
    sessionId,
  );
}

async function collectState(client, sessionId) {
  return await evaluate(client, STATE_SOURCE, sessionId);
}

async function clickControl(client, sessionId, control) {
  const x = Math.round(control.x + control.width / 2);
  const y = Math.round(control.y + control.height / 2);
  for (const event of [
    { type: "mouseMoved", button: "none", buttons: 0 },
    { type: "mousePressed", button: "left", buttons: 1, clickCount: 1 },
    { type: "mouseReleased", button: "left", buttons: 0, clickCount: 1 },
  ]) {
    await client.send(
      "Input.dispatchMouseEvent",
      { ...event, x, y },
      sessionId,
    );
  }
  return { x, y, coordinateSpace: "pdf-extension-child-target-viewport" };
}

function traceForAction(trace, actionId, startLength) {
  return (trace || [])
    .slice(startLength)
    .filter((entry) => entry.actionId === actionId);
}

function hasEvent(entries, eventType) {
  return entries.some(
    (entry) => entry.type === "event" && entry.eventType === eventType,
  );
}

function hasMethod(entries, fragment) {
  return entries.some(
    (entry) => entry.type === "method-enter" && entry.label?.includes(fragment),
  );
}

function hasThrow(entries) {
  return entries.find((entry) => entry.type === "method-throw") || null;
}

function stateChanged(before, after) {
  return (
    JSON.stringify(before?.value || null) !==
    JSON.stringify(after?.value || null)
  );
}

function classify(
  kind,
  entries,
  beforeState,
  afterState,
  setup,
  screenshotChanged,
) {
  const eventName = {
    fit: "fit-to-changed",
    zoomIn: "zoom-in",
    zoomOut: "zoom-out",
    rotate: "rotate-left",
  }[kind];
  const toolbarMethod = {
    fit: "onFitToButtonClick_",
    zoomIn: "onZoomInClick_",
    zoomOut: "onZoomOutClick_",
    rotate: "onRotateClick_",
  }[kind];
  const viewerMethod = {
    fit: "onFitToChanged",
    zoomIn: "onZoomIn",
    zoomOut: "onZoomOut",
    rotate: "onRotateLeft_",
  }[kind];
  const actionMethod = {
    fit: "setFittingType",
    zoomIn: "zoomIn",
    zoomOut: "zoomOut",
    rotate: "rotateCounterclockwise",
  }[kind];
  const thrown = hasThrow(entries);
  const rawStateChanged = stateChanged(beforeState, afterState);
  const successfulStateChanged = !thrown && rawStateChanged;
  const incidentalStateChangedAfterThrow = !!thrown && rawStateChanged;
  const changedOrVisible =
    successfulStateChanged || (!thrown && screenshotChanged);
  const control = setup?.controls?.[kind];
  const data = {
    controlFound: !!control && control.width > 0 && control.height > 0,
    clickObserved: hasEvent(entries, "click"),
    toolbarHandler: hasMethod(entries, toolbarMethod),
    customEvent: hasEvent(entries, eventName),
    viewerHandler: hasMethod(entries, viewerMethod),
    actionMethod: hasMethod(entries, actionMethod),
    stateChanged: successfulStateChanged,
    rawStateChanged,
    incidentalStateChangedAfterThrow,
    screenshotChanged,
    thrown,
  };
  if (!data.controlFound) data.firstFailingHop = "control-not-found";
  else if (!data.clickObserved) data.firstFailingHop = "click-not-observed";
  else if (!data.toolbarHandler && !data.customEvent && !data.actionMethod)
    data.firstFailingHop = "toolbar-handler-not-called";
  else if (!data.customEvent)
    data.firstFailingHop = "custom-event-not-dispatched";
  else if (!data.viewerHandler && !data.actionMethod)
    data.firstFailingHop = "viewer-handler-not-called";
  else if (!data.actionMethod) {
    data.firstFailingHop =
      kind === "rotate"
        ? "controller-method-not-called"
        : "viewport-method-not-called";
  } else if (thrown) data.firstFailingHop = "method-threw";
  else if (!changedOrVisible)
    data.firstFailingHop = "state-did-not-change-after-method";
  else data.firstFailingHop = "no-failure-observed";
  return data;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  fs.mkdirSync(args.outDir, { recursive: true });
  const summary = {
    devtoolsPort: args.devtoolsPort,
    urlContains: args.urlContains,
  };
  let client = null;
  try {
    const target = await pollTarget(args, summary);
    summary.selectedTarget = {
      id: target.id,
      type: target.type,
      url: target.url,
      title: target.title,
    };
    client = connectDevTools(target.webSocketDebuggerUrl);
    await client.open;
    for (const domain of ["Page", "Runtime", "DOM", "Input", "Target"]) {
      summary[`${domain}Enabled`] = await safeSend(client, `${domain}.enable`);
    }
    await safeSend(client, "Target.setAutoAttach", {
      autoAttach: true,
      waitForDebuggerOnStart: false,
      flatten: true,
    });
    await safeSend(client, "Page.bringToFront");
    await sleep(args.settleSeconds * 1000);
    summary.childTargets = client.events
      .filter((event) => event.method === "Target.attachedToTarget")
      .map((event) => ({
        sessionId: event.params.sessionId,
        targetInfo: event.params.targetInfo,
      }));
    const pdfChild = summary.childTargets.find((child) =>
      child.targetInfo?.url?.includes("chrome-extension://"),
    );
    if (!pdfChild) {
      throw new Error("missing PDF extension child target");
    }
    summary.pdfChild = pdfChild;
    await safeSend(client, "Runtime.enable", {}, pdfChild.sessionId);
    await safeSend(client, "Page.enable", {}, pdfChild.sessionId);
    await safeSend(client, "DOM.enable", {}, pdfChild.sessionId);

    const setupResult = await evaluate(
      client,
      SETUP_SOURCE,
      pdfChild.sessionId,
    );
    summary.setup = setupResult;
    writeJson(path.join(args.outDir, "setup.json"), setupResult);
    if (!setupResult.ok) {
      throw new Error(`setup failed: ${setupResult.error}`);
    }
    const setup = setupResult.value;
    const resourcesPrivateProbe = await evaluate(
      client,
      RESOURCES_PRIVATE_SOURCE,
      pdfChild.sessionId,
    );
    summary.resourcesPrivateProbe = resourcesPrivateProbe;
    writeJson(
      path.join(args.outDir, "resources-private-probe.json"),
      resourcesPrivateProbe,
    );
    summary.results = [];
    for (const kind of ["fit", "zoomIn", "zoomOut", "rotate"]) {
      const actionId = `${kind}-${Date.now()}`;
      const beforeTrace = await snapshotTrace(client, pdfChild.sessionId);
      const startLength = beforeTrace.value?.trace?.length || 0;
      await setAction(client, pdfChild.sessionId, actionId);
      const beforeState = await collectState(client, pdfChild.sessionId);
      const beforeScreenshot = await captureScreenshot(
        client,
        args,
        `${kind}-before.png`,
      );
      const control = setup.controls?.[kind];
      const activation = control
        ? await clickControl(client, pdfChild.sessionId, control)
        : null;
      await sleep(args.actionSettleMs);
      const afterState = await collectState(client, pdfChild.sessionId);
      const afterScreenshot = await captureScreenshot(
        client,
        args,
        `${kind}-after.png`,
      );
      const afterTrace = await snapshotTrace(client, pdfChild.sessionId);
      const entries = traceForAction(
        afterTrace.value?.trace || [],
        actionId,
        startLength,
      );
      const classification = classify(
        kind,
        entries,
        beforeState,
        afterState,
        setup,
        beforeScreenshot.sha256 !== afterScreenshot.sha256,
      );
      const result = {
        feature: kind,
        actionId,
        activation,
        beforeState,
        afterState,
        screenshots: [
          beforeScreenshot.relativePath,
          afterScreenshot.relativePath,
        ],
        traceEntries: entries,
        ...classification,
      };
      summary.results.push(result);
      writeJson(path.join(args.outDir, `${kind}.json`), result);
    }
    summary.status = summary.results.every(
      (result) => result.firstFailingHop === "no-failure-observed",
    )
      ? "pass"
      : "partial";
  } catch (error) {
    summary.status = "error";
    summary.error = String(error.stack || error);
    throw error;
  } finally {
    writeJson(path.join(args.outDir, "toolbar-events-summary.json"), summary);
    if (client) client.socket.close();
  }
}

main().catch((error) => {
  console.error(error.stack || error);
  process.exit(1);
});
