// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "TwoProfiles",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(
            name: "TwoProfiles",
            path: "Sources/TwoProfiles",
            exclude: ["Shaders.metal", "shaders.metallib"],
            linkerSettings: [
                .linkedFramework("Cocoa"),
                .linkedFramework("Metal"),
                .linkedFramework("QuartzCore"),
                .linkedFramework("IOSurface"),
            ]
        )
    ]
)
