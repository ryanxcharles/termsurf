//
//  TermSurfTitleUITests.swift
//  TermSurfUITests
//
//  Created by luca on 13.10.2025.
//

import XCTest

final class TermSurfTitleUITests: TermSurfCustomConfigCase {
    override func setUp() async throws {
        try await super.setUp()
        try updateConfig(#"title = "TermSurfUITestsLaunchTests""#)
    }

    @MainActor
    func testTitle() throws {
        let app = try termsurfApplication()
        app.launch()

        XCTAssertEqual(app.windows.firstMatch.title, "TermSurfUITestsLaunchTests", "Oops, `title=` doesn't work!")
    }
}
