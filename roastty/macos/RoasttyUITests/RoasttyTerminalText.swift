//
//  RoasttyTerminalText.swift
//  Roastty
//
//  Created by Codex on 12.06.2026.
//

import XCTest

extension XCUIApplication {
    @MainActor func roasttyTerminalText(in terminal: XCUIElement) -> String {
        var values: [String] = []
        appendTerminalValue(from: terminal, to: &values)

        for textView in textViews.allElementsBoundByIndex where textView.exists {
            appendTerminalValue(from: textView, to: &values)
        }

        return values.joined(separator: "\n")
    }

    @MainActor func roasttyTerminalSnapshot(in terminal: XCUIElement) -> String {
        var lines = ["terminal.value=\(terminal.value as? String ?? "<nil>")"]
        let textViewElements = textViews.allElementsBoundByIndex
        lines.append("textViews.count=\(textViewElements.count)")

        for (index, textView) in textViewElements.enumerated() {
            let value = textView.exists ? (textView.value as? String ?? "<nil>") : "<missing>"
            lines.append("textView[\(index)].exists=\(textView.exists)")
            lines.append("textView[\(index)].value=\(value)")
        }

        lines.append("app.debugDescription=\n\(debugDescription)")
        return lines.joined(separator: "\n")
    }

    @MainActor private func appendTerminalValue(from element: XCUIElement, to values: inout [String]) {
        if let value = element.value as? String, !value.isEmpty {
            values.append(value)
        }

        if !element.label.isEmpty {
            values.append(element.label)
        }
    }
}
