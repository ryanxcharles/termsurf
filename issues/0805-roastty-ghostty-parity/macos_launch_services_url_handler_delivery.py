#!/usr/bin/env python3
"""Live Launch Services URL handler delivery guard for Issue 805 Experiment 194."""

from __future__ import annotations

import json
import subprocess
import tempfile
import textwrap
import time
import uuid
from pathlib import Path

from macos_window_padding_pixel_runtime import (
    APP,
    ROOT,
    crash_reports,
    quote_applescript,
    require,
    run_osascript,
    scoped_pids,
    terminate_process,
    wait_for_app,
    wait_for_crash_report_settle,
)


LOGS = ROOT / "logs"
LATEST_JSON = LOGS / "issue805-exp194-launch-services-latest.json"
LSREGISTER = Path(
    "/System/Library/Frameworks/CoreServices.framework/Frameworks/"
    "LaunchServices.framework/Support/lsregister"
)


def run_checked(command: list[str], *, cwd: Path | None = None, timeout: int = 30) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=cwd or ROOT,
        text=True,
        capture_output=True,
        timeout=timeout,
    )
    require(
        result.returncode == 0,
        f"command failed: {' '.join(command)}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}",
    )
    return result


def read(path: Path) -> str:
    if not path.exists():
        return ""
    return path.read_text(errors="replace")


def wait_for_delivery(path: Path, expected_url: str, description: str, timeout: float = 15.0) -> None:
    deadline = time.monotonic() + timeout
    last = ""
    while time.monotonic() < deadline:
        last = read(path).strip()
        if last == expected_url:
            return
        time.sleep(0.25)
    raise AssertionError(f"{description} delivery missing: expected={expected_url!r} actual={last!r}")


def wait_for_trace(trace: Path, expected_url: str, timeout: float = 10.0) -> str:
    deadline = time.monotonic() + timeout
    trace_text = ""
    while time.monotonic() < deadline:
        trace_text = read(trace)
        if f"openURL url={expected_url}" in trace_text:
            require("openURL suppressed=true" not in trace_text, "Roastty URL path was suppressed")
            return trace_text
        time.sleep(0.25)
    raise AssertionError(f"Roastty URL trace missing for {expected_url}\ntrace:\n{trace_text}")


def write_config(path: Path) -> None:
    path.write_text("macos-applescript = true\n")


def build_handler_app(handler_app: Path, delivered: Path, scheme: str, bundle_id: str) -> None:
    contents = handler_app / "Contents"
    macos = contents / "MacOS"
    macos.mkdir(parents=True)
    executable = macos / "Issue805Exp194URLHandler"
    source = handler_app.parent / "Issue805Exp194URLHandler.swift"
    source.write_text(
        f"""
        import Cocoa
        import Foundation

        final class Delegate: NSObject, NSApplicationDelegate {{
            private let deliveredPath = {json.dumps(str(delivered))}

            func applicationDidFinishLaunching(_ notification: Notification) {{
                NSApp.setActivationPolicy(.accessory)
                DispatchQueue.main.asyncAfter(deadline: .now() + 10.0) {{
                    NSApp.terminate(nil)
                }}
            }}

            func application(_ application: NSApplication, open urls: [URL]) {{
                let text = urls.map {{ $0.absoluteString }}.joined(separator: "\\n") + "\\n"
                let url = URL(fileURLWithPath: deliveredPath)
                try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
                try? text.write(to: url, atomically: true, encoding: .utf8)
                NSApp.terminate(nil)
            }}
        }}

        let app = NSApplication.shared
        let delegate = Delegate()
        app.delegate = delegate
        app.run()
        """
    )

    run_checked(["swiftc", str(source), "-o", str(executable), "-framework", "Cocoa"], timeout=60)
    plist = contents / "Info.plist"
    plist.write_text(
        textwrap.dedent(
            f"""
            <?xml version="1.0" encoding="UTF-8"?>
            <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
            <plist version="1.0">
            <dict>
              <key>CFBundleDevelopmentRegion</key>
              <string>en</string>
              <key>CFBundleExecutable</key>
              <string>{executable.name}</string>
              <key>CFBundleIdentifier</key>
              <string>{bundle_id}</string>
              <key>CFBundleName</key>
              <string>Issue805Exp194URLHandler</string>
              <key>CFBundlePackageType</key>
              <string>APPL</string>
            </dict>
            </plist>
            """
        ).lstrip()
    )
    run_checked(["/usr/libexec/PlistBuddy", "-c", "Add :CFBundleURLTypes array", str(plist)])
    run_checked(["/usr/libexec/PlistBuddy", "-c", "Add :CFBundleURLTypes:0 dict", str(plist)])
    run_checked(["/usr/libexec/PlistBuddy", "-c", "Add :CFBundleURLTypes:0:CFBundleTypeRole string Viewer", str(plist)])
    run_checked(["/usr/libexec/PlistBuddy", "-c", f"Add :CFBundleURLTypes:0:CFBundleURLName string {bundle_id}", str(plist)])
    run_checked(["/usr/libexec/PlistBuddy", "-c", "Add :CFBundleURLTypes:0:CFBundleURLSchemes array", str(plist)])
    run_checked(["/usr/libexec/PlistBuddy", "-c", f"Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string {scheme}", str(plist)])
    run_checked(["/usr/bin/codesign", "--force", "--deep", "--sign", "-", str(handler_app)], timeout=30)


def set_default_handler(temp: Path, scheme: str, bundle_id: str, handler_app: Path) -> str:
    swift = temp / "set-default-handler.swift"
    swift.write_text(
        textwrap.dedent(
            """
            import CoreServices
            import Darwin
            import Foundation

            let scheme = CommandLine.arguments[1] as CFString
            let bundle = CommandLine.arguments[2] as CFString
            let appURL = URL(fileURLWithPath: CommandLine.arguments[3]) as CFURL
            let registerStatus = LSRegisterURL(appURL, true)
            let status = LSSetDefaultHandlerForURLScheme(scheme, bundle)
            let current = LSCopyDefaultHandlerForURLScheme(scheme)?.takeRetainedValue() as String?
            print("registerStatus=\\(registerStatus)")
            print("status=\\(status)")
            print("current=\\(current ?? \"\")")
            exit(registerStatus == noErr && status == noErr ? 0 : 1)
            """
        ).lstrip()
    )
    result = run_checked(["swift", str(swift), scheme, bundle_id, str(handler_app)], timeout=30)
    return result.stdout


def launch_app(config: Path, trace: Path) -> int:
    before = scoped_pids()
    require(not before, f"debug Roastty app is already running: {sorted(before)}")
    result = subprocess.run(
        [
            "open",
            "-n",
            "--env",
            f"ROASTTY_CONFIG_PATH={config}",
            "--env",
            "ROASTTY_CLEAR_USER_DEFAULTS=1",
            "--env",
            "ROASTTY_USER_DEFAULTS_SUITE=com.termsurf.roastty.issue805.exp194.launchservices",
            "--env",
            f"ROASTTY_UI_KEY_TRACE_PATH={trace}",
            "--env",
            "ROASTTY_UI_TEST_ENABLE_OPEN_URL_ACTION=1",
            str(APP),
        ],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    require(result.returncode == 0, f"open failed\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}")

    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        created = sorted(scoped_pids() - before)
        if created:
            return created[0]
        time.sleep(0.25)
    raise AssertionError("open did not start a scoped debug Roastty process")


def invoke_roastty_open(url: str) -> None:
    app_literal = quote_applescript(APP)
    escaped_url = url.replace("\\", "\\\\").replace('"', '\\"')
    script = textwrap.dedent(
        f"""
        tell application {app_literal}
          activate
          set cfg to new surface configuration from {{command:"/bin/sh -c 'sleep 60'", wait after command:true}}
          new window with configuration cfg
          delay 1
          set t0 to focused terminal of selected tab of front window
          perform action "ui_test_open_url:{escaped_url}" on t0
        end tell
        """
    )
    run_osascript(script, timeout=30)


def main() -> int:
    require(APP.is_dir(), f"app not built: {APP}")
    require(LSREGISTER.is_file(), f"lsregister missing: {LSREGISTER}")
    LOGS.mkdir(parents=True, exist_ok=True)

    token = uuid.uuid4().hex
    scheme = f"termsurf-issue805-exp194-{token}"
    bundle_id = f"com.termsurf.issue805.exp194.{token}"
    url = f"{scheme}://delivered/path?token={token}"
    evidence: dict[str, object] = {
        "scheme": scheme,
        "bundle_id": bundle_id,
        "url": url,
        "app": str(APP),
    }

    before_crashes = crash_reports()
    with tempfile.TemporaryDirectory(prefix="issue805-exp194-launch-services-", dir=LOGS) as temp_dir:
        temp = Path(temp_dir)
        handler_app = temp / "Issue805Exp194URLHandler.app"
        delivered = temp / "delivered-url.txt"
        config = temp / "config.roastty"
        trace = temp / "trace.log"

        write_config(config)
        build_handler_app(handler_app, delivered, scheme, bundle_id)
        run_checked([str(LSREGISTER), "-f", str(handler_app)], timeout=30)
        evidence["set_default_handler"] = set_default_handler(temp, scheme, bundle_id, handler_app)
        print(evidence["set_default_handler"], flush=True)
        time.sleep(2.0)

        run_checked(["open", url], timeout=30)
        wait_for_delivery(delivered, url, "direct Launch Services")
        evidence["direct_delivery"] = read(delivered).strip()
        delivered.unlink()

        pid = launch_app(config, trace)
        try:
            wait_for_app(pid)
            invoke_roastty_open(url)
            trace_text = wait_for_trace(trace, url)
            wait_for_delivery(delivered, url, "Roastty Launch Services")
            evidence["roastty_trace"] = trace_text
            evidence["roastty_delivery"] = read(delivered).strip()
        finally:
            terminate_process(pid)

    new_crashes = sorted(str(path) for path in wait_for_crash_report_settle(before_crashes))
    evidence["new_crash_reports"] = new_crashes
    require(not new_crashes, f"new Roastty crash reports: {new_crashes}")

    LATEST_JSON.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    print("macos_launch_services_url_handler_delivery=pass")
    print(json.dumps(evidence, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
