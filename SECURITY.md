# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability, **please do NOT open a public issue**.

Instead, use [GitHub's private vulnerability reporting](https://github.com/rogelioRuiz/capacitor-wasm-agent-tools/security/advisories/new) to report it. Include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

We will acknowledge your report within 48 hours and aim to provide a fix within 90 days.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.x     | Yes       |

## Security Model

This package provides sandboxed execution of WebAssembly, JavaScript, and Python via Rust FFI on mobile devices. Key security properties:

- **WASM isolation** — All guest code (WASM, JS, Python) runs inside Wasmtime's WebAssembly sandbox. Guest code cannot access host memory, the filesystem, or native APIs outside the sandbox ABI.
- **Fuel metering** — A configurable instruction budget prevents infinite loops and CPU exhaustion.
- **Memory limits** — Guest memory is capped at a configurable byte limit; exceeding it aborts execution.
- **Timeout enforcement** — A watchdog thread and epoch interruption ensure execution never exceeds the configured wall-clock timeout.
- **Blocked imports (Python)** — The Python runtime blocks `subprocess`, `socket`, and `ctypes` to prevent network access and native code execution from guest code.
- **Rust memory safety** — The native FFI layer is written in Rust, providing memory safety guarantees.
- **No filesystem access** — WASI is configured with an empty preopen list; guest code cannot read or write the host filesystem.
