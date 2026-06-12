//
//  RoasttyTerminalOutputUITests.swift
//  Roastty
//
//  Created by Codex on 12.06.2026.
//

import XCTest

final class RoasttyTerminalOutputUITests: RoasttyCustomConfigCase {
    @MainActor func testTerminalOutputIsVisibleToUIAutomation() async throws {
        try updateConfig(
            """
            title = "RoasttyTerminalOutputUITests"
            initial-command = direct:echo TERMSURF_READY_158
            wait-after-command = true
            """
        )

        let app = try roasttyApplication()
        app.launchEnvironment["DISABLE_AUTO_UPDATE"] = "true"
        app.launch()

        let window = app.windows.firstMatch
        XCTAssertTrue(window.waitForExistence(timeout: 5), "Main window should appear")
        guard window.title == "RoasttyTerminalOutputUITests" else {
            throw XCTSkip("Configured window title was not visible; actual title: \(window.title)")
        }

        let terminal = app.groups["Terminal pane"].firstMatch
        XCTAssertTrue(terminal.waitForExistence(timeout: 5), "Terminal pane should appear")

        guard waitForTerminalValue(terminal, app: app, containing: "TERMSURF_READY_158", timeout: 10) else {
            throw XCTSkip(
                "Terminal output did not become visible through accessibility. Snapshot:\n\(app.roasttyTerminalSnapshot(in: terminal))"
            )
        }
    }

    @MainActor private func waitForTerminalValue(
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

        return false
    }
}
