#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR/../rust/wasm-agent-tools-ffi"
OUT_WASM="$RUST_DIR/wasm/rustpython.wasm"
TARGET_DIR="$RUST_DIR/target/rustpython-build"

RUSTPY_DIR="$(find "$HOME/.cargo/registry/src" -maxdepth 2 -type d -name 'rustpython-0.4.0' | head -1)"
if [[ -z "$RUSTPY_DIR" ]]; then
  cargo info rustpython >/dev/null
  RUSTPY_DIR="$(find "$HOME/.cargo/registry/src" -maxdepth 2 -type d -name 'rustpython-0.4.0' | head -1)"
fi

if [[ -z "$RUSTPY_DIR" ]]; then
  echo "Could not locate rustpython-0.4.0 source in cargo registry" >&2
  exit 1
fi

cargo build \
  --release \
  --target wasm32-wasip1 \
  --no-default-features \
  --features freeze-stdlib,threading,importlib \
  --manifest-path "$RUSTPY_DIR/Cargo.toml" \
  --target-dir "$TARGET_DIR"

cp "$TARGET_DIR/wasm32-wasip1/release/rustpython.wasm" "$OUT_WASM"
echo "Wrote $OUT_WASM"
