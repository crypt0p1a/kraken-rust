// swift-tools-version:5.9
// Swift package of the Kraken Spot REST client: the bindings of kraken-ffi (scripts/bindings.sh)
//
//   scripts/build.sh ios macos && scripts/bindings.sh swift xcframework   (macOS)
//   swift test                                                            (macOS host)
//   xcodebuild test -scheme Kraken -destination 'platform=iOS Simulator,name=...'
//
// On Linux, the C module is the shared library of the host instead of the XCFramework:
//   scripts/bindings.sh swift && swift test -Xlinker -L../../target/debug
import PackageDescription

#if os(Linux)
let ffi: Target = .systemLibrary(name: "KrakenFFI", path: "Sources/KrakenFFI")
#else
let ffi: Target = .binaryTarget(name: "KrakenFFI", path: "KrakenFFI.xcframework")
#endif

let package = Package(
    name: "Kraken",
    platforms: [.iOS(.v13), .macOS(.v11)],
    products: [
        .library(name: "Kraken", targets: ["Kraken"]),
    ],
    targets: [
        ffi,
        // the Swift sources generated for kraken-network, kraken-spot-api and kraken-ffi
        .target(name: "Kraken", dependencies: ["KrakenFFI"], path: "Sources/Kraken"),
        .testTarget(name: "KrakenTests", dependencies: ["Kraken"]),
    ]
)
