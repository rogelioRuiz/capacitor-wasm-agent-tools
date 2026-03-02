#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR/../rust/wasm-agent-tools-ffi"
PLUGIN_DIR="$SCRIPT_DIR/.."

: "${ANDROID_NDK_HOME:=$HOME/Android/Sdk/ndk/27.0.12077973}"
export ANDROID_NDK_HOME

cd "$RUST_DIR"

echo "==> Building Android arm64-v8a library..."
cargo ndk -t arm64-v8a build --release

echo "==> Copying libwasm_agent_tools_ffi.so into jniLibs..."
mkdir -p "$PLUGIN_DIR/android/src/main/jniLibs/arm64-v8a"
cp "$RUST_DIR/target/aarch64-linux-android/release/libwasm_agent_tools_ffi.so" \
  "$PLUGIN_DIR/android/src/main/jniLibs/arm64-v8a/"

echo "==> Regenerating Kotlin UniFFI bindings..."
cargo build
cargo run --bin uniffi-bindgen -- generate \
  --library "$RUST_DIR/target/debug/libwasm_agent_tools_ffi.so" \
  --language kotlin \
  --out-dir "$PLUGIN_DIR/android/src/main/java/"

echo "==> Android build complete."
