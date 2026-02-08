// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "termsurf-window",
    platforms: [.macOS(.v13)],
    targets: [
        .executableTarget(
            name: "termsurf-window",
            path: "Sources"
        ),
    ]
)
