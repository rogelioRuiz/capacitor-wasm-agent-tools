# capacitor-wasm-agent-tools

## 0.1.1

### Patch Changes

- Add README, LICENSE, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT, CHANGELOG, and GitHub issue/PR templates for open-source release
- Add `repository`, `homepage`, `bugs` fields and expand keywords in package.json

## 0.1.0

### Initial Release

- WASM sandbox with custom ABI (`alloc`/`execute`/`memory` exports), fuel metering, memory limits, and timeout enforcement via Wasmtime 29
- JavaScript runtime via embedded QuickJS engine (compiled to WASM), executed over WASI
- Python runtime via embedded RustPython engine (compiled to WASM), executed over WASI, with `subprocess`/`socket`/`ctypes` blocked
- Configurable resource limits per execution: `fuelLimit`, `maxMemoryBytes`, `timeoutSecs`
- Android (arm64-v8a) and iOS (arm64 device + arm64 simulator) support via UniFFI 0.28
- 25-test E2E harness for Android and iOS
