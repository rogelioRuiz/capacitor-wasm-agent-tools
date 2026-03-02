// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "CapacitorWasmAgentTools",
    platforms: [.iOS(.v14)],
    products: [
        .library(
            name: "CapacitorWasmAgentTools",
            targets: ["WasmAgentToolsPlugin"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/ionic-team/capacitor-swift-pm.git", from: "8.0.0")
    ],
    targets: [
        .binaryTarget(
            name: "WasmAgentToolsFFI",
            path: "ios/Frameworks/WasmAgentToolsFFI.xcframework"
        ),
        .target(
            name: "WasmAgentToolsPlugin",
            dependencies: [
                .product(name: "Capacitor", package: "capacitor-swift-pm"),
                .product(name: "Cordova", package: "capacitor-swift-pm"),
                "WasmAgentToolsFFI"
            ],
            path: "ios/Sources/WasmAgentToolsPlugin",
            exclude: [
                "Generated/wasm_agent_tools_ffiFFI.h",
                "Generated/wasm_agent_tools_ffiFFI.modulemap"
            ]
        )
    ]
)
