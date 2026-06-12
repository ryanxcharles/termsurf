//
//  RoasttyTitleUITests.swift
//  RoasttyUITests
//
//  Created by luca on 13.10.2025.
//

import XCTest

final class RoasttyTitleUITests: RoasttyCustomConfigCase {
    override func setUpWithError() throws {
        try super.setUpWithError()
        try updateConfig(#"title = "RoasttyUITestsLaunchTests""#)
    }

    @MainActor
    func testTitle() throws {
        let app = try roasttyApplication()
        app.launch()

        let window = app.windows.firstMatch
        XCTAssertTrue(window.waitForExistence(timeout: 5), "Main window should appear")
        XCTAssertTrue(
            waitForWindowTitle(app, "RoasttyUITestsLaunchTests", timeout: 5),
            "Configured title should be visible. Window titles: \(windowTitles(app))\n\(app.debugDescription)"
        )
    }

    @MainActor private func waitForWindowTitle(
        _ app: XCUIApplication,
        _ expected: String,
        timeout: TimeInterval
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if windowTitles(app).contains(expected) {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.1))
        }

        return false
    }

    @MainActor private func windowTitles(_ app: XCUIApplication) -> [String] {
        app.windows.allElementsBoundByIndex.map(\.title)
    }
}
