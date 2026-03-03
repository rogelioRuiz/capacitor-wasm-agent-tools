# capacitor-wasm-agent-tools

[![npm version](https://img.shields.io/npm/v/capacitor-wasm-agent-tools)](https://www.npmjs.com/package/capacitor-wasm-agent-tools)
[![CI](https://github.com/rogelioRuiz/capacitor-wasm-agent-tools/actions/workflows/ci.yml/badge.svg)](https://github.com/rogelioRuiz/capacitor-wasm-agent-tools/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

Standalone Rust-backed Capacitor plugin for secure, sandboxed execution of WebAssembly, JavaScript, and Python on native mobile platforms. Powered by [Wasmtime](https://wasmtime.dev/), [QuickJS](https://bellard.org/quickjs/), and [RustPython](https://rustpython.github.io/) — all running inside WASM with strict resource limits.

## Features

- **WASM sandbox** — execute raw WASM binaries or WAT text with fuel metering, memory limits, and timeout enforcement
- **JavaScript runtime** — run arbitrary JS code via embedded QuickJS engine (compiled to WASM)
- **Python runtime** — run Python code via embedded RustPython engine (compiled to WASM), with dangerous imports blocked
- **Resource limits** — configurable fuel budget, max memory, and timeout per execution
- **Zero native code execution** — all guest code runs inside WASM; no native eval
- **Cross-platform** — Android (arm64) and iOS (arm64 device + simulator)

## Install

```bash
npm install capacitor-wasm-agent-tools
npx cap sync
```

## Quick Start

### WASM Sandbox

Execute a WebAssembly binary that implements the sandbox ABI (`alloc`, `execute`, `memory` exports):

```typescript
import { WasmAgentTools } from 'capacitor-wasm-agent-tools'

await WasmAgentTools.createWasmSandbox()

const result = await WasmAgentTools.executeSandbox({
  wasmBase64: '<base64-encoded .wasm binary>',
  inputJson: '{"key":"value"}',
  config: { fuelLimit: 1_000_000, maxMemoryBytes: 16_777_216, timeoutSecs: 30 },
})
console.log(result.output)        // JSON string returned by the WASM module
console.log(result.fuelConsumed)  // instructions executed
console.log(result.durationMs)    // wall-clock time
```

Alternatively, use the WAT text format:

```typescript
const result = await WasmAgentTools.executeWatSandbox({
  watText: `(module ...)`,
  inputJson: '{}',
})
```

### JavaScript Runtime

```typescript
await WasmAgentTools.createJsRuntime()

const result = await WasmAgentTools.executeJs({
  code: `
    const x = [1, 2, 3].map(n => n * 2)
    console.log(JSON.stringify(x))
  `,
  config: { timeoutSecs: 5 },
})
console.log(result.output)  // "[2,4,6]\n"
console.log(result.error)   // null (or error message if execution failed)
```

### Python Runtime

```typescript
await WasmAgentTools.createPythonRuntime()

const result = await WasmAgentTools.executePython({
  code: `
import json, math
data = {"pi": math.pi, "sqrt2": math.sqrt(2)}
print(json.dumps(data))
  `,
  config: { timeoutSecs: 5 },
})
console.log(result.output)  // '{"pi": 3.141592653589793, "sqrt2": 1.4142135623730951}\n'
```

Dangerous imports are blocked (`subprocess`, `socket`, `ctypes`). Standard library modules (`math`, `json`, `re`, etc.) are available.

## API

### WasmAgentToolsPlugin

| Method | Description |
|--------|-------------|
| `createWasmSandbox()` | Initialize the WASM sandbox engine |
| `executeSandbox({ wasmBase64, inputJson, config? })` | Execute a WASM binary (base64-encoded) |
| `executeWatSandbox({ watText, inputJson, config? })` | Execute a WAT text module |
| `createJsRuntime()` | Initialize the QuickJS JavaScript runtime |
| `executeJs({ code, config? })` | Execute JavaScript code |
| `createPythonRuntime()` | Initialize the RustPython runtime |
| `executePython({ code, config? })` | Execute Python code |

### SandboxConfig / JsConfig / PythonConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fuelLimit` | `number` | 1,000,000 (WASM) / 1,000,000,000 (JS/Python) | Max instructions before abort |
| `maxMemoryBytes` | `number` | 16,777,216 (WASM) / 33,554,432 (JS/Python) | Max guest memory in bytes |
| `timeoutSecs` | `number` | 30 (WASM) / 5 (JS/Python) | Wall-clock timeout in seconds |

### WASM Sandbox ABI

WASM modules must export:

| Export | Type | Description |
|--------|------|-------------|
| `memory` | Memory | Linear memory shared with host |
| `alloc(size: i32) -> i32` | Function | Allocate `size` bytes; returns pointer |
| `execute(ptr: i32, len: i32) -> i64` | Function | Run with input at `[ptr, ptr+len)`. Returns `(out_ptr << 32) \| out_len` |

The sandbox reads the input JSON from guest memory and reads back the output JSON using the packed pointer/length return value.

## E2E Tests

25/25 tests passing on Android and iOS — 9 WASM sandbox tests, 8 JavaScript tests, 8 Python tests:

| Suite | Tests | Coverage |
|-------|-------|----------|
| WASM Sandbox | 9 | create, echo, fuel limit, timeout, fuel tracking, invalid WASM, missing exports, WAT, memory limit |
| JavaScript | 8 | create, expression, console.log, console.error, multi-statement, timeout, throw, no require |
| Python | 8 | create, print, multi-print, math, json, timeout, blocked imports, raise |

## Platforms

| Platform | Architecture | Binary |
|----------|-------------|--------|
| Android | arm64-v8a | `libwasm_agent_tools_ffi.so` |
| iOS | arm64 (device) | `WasmAgentToolsFFI.xcframework` |
| iOS | arm64 (Apple Silicon sim) | `WasmAgentToolsFFI.xcframework` |

Prebuilt native binaries are included in the npm package. No Rust toolchain required for consumers.

## Building Native Binaries

For contributors modifying the Rust FFI code:

```bash
# Android
./scripts/build-android.sh

# iOS (run on macOS with Xcode)
./scripts/build-ios.sh
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for full toolchain requirements.

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

## Security

To report vulnerabilities, see [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) &copy; 2025-present Techxagon
