# Contributing to capacitor-wasm-agent-tools

Thanks for your interest in contributing! This guide will help you get set up and understand our development workflow.

## Prerequisites

- **Node.js** >= 20
- **npm** (comes with Node.js)
- **Git**

For building native binaries (optional — prebuilt binaries are included):

- **Rust** with targets: `aarch64-linux-android`, `aarch64-apple-ios`, `aarch64-apple-ios-sim`
- **cargo-ndk** v4+ (Android)
- **Android NDK** (Android)
- **Xcode** 15+ with command-line tools (iOS, macOS only)

## Getting Started

```bash
# Clone the repo
git clone https://github.com/rogelioRuiz/capacitor-wasm-agent-tools.git
cd capacitor-wasm-agent-tools

# Install dependencies
npm install

# Build
npm run build
```

## Project Structure

```
src/
  index.ts              # Package entry point — re-exports plugin and types
  definitions.ts        # TypeScript interfaces (WasmAgentToolsPlugin, configs, results)
  web.ts                # Web stub (throws "native only" for all methods)
rust/
  wasm-agent-tools-ffi/ # Rust FFI library (UniFFI 0.28, Wasmtime 29)
    src/
      lib.rs            # Crate root — error types, module declarations
      engine.rs         # Shared Wasmtime engine, watchdog, store builders
      sandbox.rs        # WasmSandbox — custom ABI execution
      js_runtime.rs     # JsRuntime — QuickJS via WASI
      python_runtime.rs # PythonRuntime — RustPython via WASI
      wasi_executor.rs  # WASI execution helper
    wasm/
      quickjs.wasm      # Embedded QuickJS JS engine
      rustpython.wasm   # Embedded RustPython interpreter
android/
  src/main/java/        # Kotlin Capacitor plugin
  src/main/jniLibs/     # Prebuilt .so binaries (arm64-v8a)
ios/
  Sources/              # Swift Capacitor plugin + UniFFI bindings
  Frameworks/           # Prebuilt xcframework (device + simulator)
scripts/
  build-android.sh      # Android native build script
  build-ios.sh          # iOS native build script
example/
  www/                  # Test app HTML + JS
  test-e2e-android.mjs  # Android E2E test runner
  test-e2e-ios.mjs      # iOS E2E test runner
```

## Development Commands

| Command | Description |
|---------|-------------|
| `npm run build` | Compile TypeScript to `dist/esm/` |
| `npm run build:watch` | Compile in watch mode |
| `npm run typecheck` | Type-check without emitting |
| `npm run clean` | Remove `dist/` directory |
| `npm run build:android` | Build Android native binaries (requires Rust + NDK) |
| `npm run build:ios` | Build iOS native binaries (requires Xcode, macOS only) |

## Running E2E Tests

The `example/` directory contains a dedicated test app covering all plugin methods. Tests run automatically on app launch and report results via HTTP to a local Node.js server.

### Android

Requires an Android device connected via USB (or an emulator).

```bash
cd example

# Run the full suite — builds APK, installs, launches, collects results
node test-e2e-android.mjs
```

The runner:
1. Builds the debug APK (`./gradlew assembleDebug`)
2. Sets up `adb reverse tcp:8099 tcp:8099` so the device can POST to the host
3. Installs and launches the app
4. Starts an HTTP server and waits for the app to POST all test results

Expected output: **25/25 passed** (9 WASM + 8 JS + 8 Python).

Set `ADB_PATH` or `ANDROID_SERIAL` to target a specific device:

```bash
ANDROID_SERIAL=emulator-5554 node test-e2e-android.mjs
```

### iOS Simulator

Requires macOS with Xcode and an Apple Silicon Mac (the prebuilt xcframework targets arm64 simulator).

```bash
cd example
node test-e2e-ios.mjs
```

The runner:
1. Copies web assets to the Xcode project
2. Builds for the simulator with `xcodebuild`
3. Installs the fresh build on the simulator
4. Starts an HTTP server (`127.0.0.1:8099`) and launches the app
5. The iOS Simulator shares the Mac's loopback — the app POSTs results to `http://127.0.0.1:8099`

Expected output: **25/25 passed** (9 WASM + 8 JS + 8 Python).

## Building Native Binaries

Prebuilt binaries are included in the repo. You only need to rebuild if you modify the Rust FFI code.

### Android

```bash
./scripts/build-android.sh
```

Requires `cargo-ndk` (v4+) and Android NDK. Produces:
- `android/src/main/jniLibs/arm64-v8a/libwasm_agent_tools_ffi.so`
- `android/src/main/java/uniffi/` (generated Kotlin bindings)

Set `ANDROID_NDK_HOME` if your NDK is not in the default location (`$HOME/Android/Sdk/ndk/27.0.12077973`).

### iOS

```bash
./scripts/build-ios.sh
```

Requires Xcode on macOS. Produces:
- `ios/Frameworks/WasmAgentToolsFFI.xcframework` (device + simulator)
- `ios/Sources/WasmAgentToolsPlugin/Generated/` (generated Swift bindings)

> **Note:** The iOS build must run on macOS. Cross-compilation from Linux is not supported for iOS targets.

### Regenerating UniFFI Bindings

After modifying the Rust API surface (`lib.rs` exports), regenerate the language bindings:

```bash
# From rust/wasm-agent-tools-ffi/
cargo build  # debug build required — release strips metadata symbols

# Kotlin
cargo run --bin uniffi-bindgen -- generate \
  --library target/debug/libwasm_agent_tools_ffi.so \
  --language kotlin \
  --out-dir ../../android/src/main/java/

# Swift
cargo run --bin uniffi-bindgen -- generate \
  --library target/debug/libwasm_agent_tools_ffi.dylib \
  --language swift \
  --out-dir ../../ios/Sources/WasmAgentToolsPlugin/Generated/
```

## Making Changes

### 1. Create a branch

```bash
git checkout -b feat/my-feature
```

### 2. Make your changes

- Follow existing code conventions (TypeScript strict, ESM, single quotes, no semicolons)
- Rust code follows standard `rustfmt` formatting

### 3. Add a changeset

If your change affects the published npm package, add a changeset:

```bash
npx changeset
```

This prompts you to describe the change and select a semver bump. The changeset file is committed with your PR and used to generate changelog entries on release.

Skip this step for docs-only or CI-only changes.

### 4. Commit with a conventional message

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(sandbox): add batch execution method
fix(python): handle empty output correctly
docs: update WASM ABI documentation
chore(deps): update wasmtime to 30
```

Scopes: `sandbox`, `js`, `python`, `engine`, `rust`, `android`, `ios`, `docs`, `ci`, `deps`

### 5. Open a pull request

Push your branch and open a PR against `main`. The CI pipeline will run typecheck and build automatically.

## Reporting Issues

- **Bugs**: Use the [bug report template](https://github.com/rogelioRuiz/capacitor-wasm-agent-tools/issues/new?template=bug_report.yml)
- **Features**: Use the [feature request template](https://github.com/rogelioRuiz/capacitor-wasm-agent-tools/issues/new?template=feature_request.yml)
- **Security**: See [SECURITY.md](SECURITY.md) — do NOT use public issues for vulnerabilities

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). Please be respectful and constructive.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
