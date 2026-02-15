// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "OneProfile",
    platforms: [.macOS(.v14)],
    targets: [
        .executableTarget(
            name: "OneProfile",
            path: "Sources/OneProfile",
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
