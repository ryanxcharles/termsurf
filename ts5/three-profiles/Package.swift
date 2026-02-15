// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ThreeProfiles",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(
            name: "ThreeProfiles",
            path: "Sources/ThreeProfiles",
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
