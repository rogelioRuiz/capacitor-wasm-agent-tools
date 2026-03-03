#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR/../rust/wasm-agent-tools-ffi"
PLUGIN_DIR="$SCRIPT_DIR/.."
GENERATED_DIR="$PLUGIN_DIR/ios/Sources/WasmAgentToolsPlugin/Generated"
XCFRAMEWORK_DIR="$PLUGIN_DIR/ios/Frameworks/WasmAgentToolsFFI.xcframework"

cd "$RUST_DIR"

echo "==> Building iOS device library..."
cargo build --release --target aarch64-apple-ios

echo "==> Building iOS simulator library..."
cargo build --release --target aarch64-apple-ios-sim

echo "==> Regenerating Swift UniFFI bindings..."
cargo build
cargo run --bin uniffi-bindgen -- generate \
  --library "$RUST_DIR/target/debug/libwasm_agent_tools_ffi.dylib" \
  --language swift \
  --out-dir "$GENERATED_DIR"

HEADERS_TMP="$RUST_DIR/target/xcframework-headers"
rm -rf "$HEADERS_TMP"
mkdir -p "$HEADERS_TMP/wasm_agent_tools_ffi"
cp "$GENERATED_DIR/wasm_agent_tools_ffiFFI.h" "$HEADERS_TMP/wasm_agent_tools_ffi/"
cat > "$HEADERS_TMP/wasm_agent_tools_ffi/module.modulemap" << 'EOF'
module wasm_agent_tools_ffiFFI {
    header "wasm_agent_tools_ffiFFI.h"
    export *
}
EOF

echo "==> Creating WasmAgentToolsFFI.xcframework..."
rm -rf "$XCFRAMEWORK_DIR"
xcodebuild -create-xcframework \
  -library "$RUST_DIR/target/aarch64-apple-ios/release/libwasm_agent_tools_ffi.a" \
  -headers "$HEADERS_TMP" \
  -library "$RUST_DIR/target/aarch64-apple-ios-sim/release/libwasm_agent_tools_ffi.a" \
  -headers "$HEADERS_TMP" \
  -output "$XCFRAMEWORK_DIR"

rm -rf "$HEADERS_TMP"

echo "==> iOS build complete."
