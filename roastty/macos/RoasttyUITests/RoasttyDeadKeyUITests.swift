//
//  RoasttyDeadKeyUITests.swift
//  Roastty
//
//  Created by Codex on 12.06.2026.
//

import AppKit
import XCTest

final class RoasttyDeadKeyUITests: RoasttyCustomConfigCase {
    @MainActor func testDeadKeyCompositionCommitsText() async throws {
        try updateConfig(
            """
            title = "RoasttyDeadKeyUITests"
            macos-option-as-alt = false
            initial-command = shell:stty -echo -icanon min 2 time 0; dd bs=2 count=1 2>/dev/null; sleep 1
            """
        )

        let traceFile = FileManager.default.temporaryDirectory
            .appendingPathComponent("RoasttyDeadKeyUITests.trace")
            .appendingPathExtension("log")
        try? FileManager.default.removeItem(at: traceFile)

        let app = try roasttyApplication()
        app.launchEnvironment["ROASTTY_UI_KEY_TRACE_PATH"] = traceFile.path
        app.launchEnvironment["DISABLE_AUTO_UPDATE"] = "true"
        app.launch()

        let terminal = app.groups["Terminal pane"].firstMatch
        XCTAssertTrue(terminal.waitForExistence(timeout: 5), "Terminal pane should appear")
        terminal.click()

        terminal.typeKey("e", modifierFlags: [.option])
        terminal.typeKey("e", modifierFlags: [])

        let trace = waitForTrace(at: traceFile, containing: "committedPreeditText text=é", timeout: 5)
        XCTAssertGreaterThanOrEqual(
            trace.components(separatedBy: "keyDown").count - 1,
            2,
            "Trace should prove both keyDown events handled the input:\n\(trace)"
        )
        XCTAssertTrue(
            trace.contains("setMarkedText"),
            "Trace should prove AppKit composition produced marked text:\n\(trace)"
        )
        XCTAssertTrue(
            trace.contains("insertText accumulated=é"),
            "Trace should prove AppKit composition committed text:\n\(trace)"
        )
        XCTAssertTrue(
            trace.contains("committedPreeditText text=é"),
            "Trace should prove committed preedit text was sent to libroastty:\n\(trace)"
        )
        XCTAssertFalse(
            trace.contains("insertText direct=é"),
            "Composed text should not bypass keyDown accumulation:\n\(trace)"
        )

        guard waitForCommittedText(terminal, app: app, containing: "é", timeout: 5) else {
            let snapshot = app.roasttyTerminalSnapshot(in: terminal)
            XCTFail(
                "Dead-key route was exercised, but this host did not expose the committed text through terminal accessibility or copy. Trace:\n\(trace)\nTerminal snapshot:\n\(snapshot)"
            )
            return
        }

        try? FileManager.default.removeItem(at: traceFile)
    }

    @MainActor private func waitForCommittedText(
        _ terminal: XCUIElement,
        app: XCUIApplication,
        containing needle: String,
        timeout: TimeInterval
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if app.roasttyTerminalText(in: terminal).contains(needle) {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.1))
        }

        NSPasteboard.general.clearContents()
        terminal.typeKey("a", modifierFlags: [.command])
        terminal.typeKey("c", modifierFlags: [.command])

        let pasteboardDeadline = Date().addingTimeInterval(timeout)
        while Date() < pasteboardDeadline {
            if NSPasteboard.general.string(forType: .string)?.contains(needle) == true {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.1))
        }

        return false
    }

    private func waitForTrace(at url: URL, containing needle: String, timeout: TimeInterval) -> String {
        let deadline = Date().addingTimeInterval(timeout)
        var latest = ""
        while Date() < deadline {
            latest = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
            if latest.contains(needle) {
                return latest
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.1))
        }

        return latest
    }
}
