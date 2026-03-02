#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR/../rust/wasm-agent-tools-ffi"
BUILD_DIR="$RUST_DIR/target/quickjs-build"
WRAPPER_C="$RUST_DIR/c/quickjs_wasi_main.c"
OUT_WASM="$RUST_DIR/wasm/quickjs.wasm"

WASI_SDK_TAG="${WASI_SDK_TAG:-30}"
WASI_SDK_VERSION="${WASI_SDK_VERSION:-30.0}"
QUICKJS_ARCHIVE_VERSION="${QUICKJS_ARCHIVE_VERSION:-2025-09-13-2}"
QUICKJS_SOURCE_VERSION="${QUICKJS_SOURCE_VERSION:-2025-09-13}"

host_archive_name() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "${os}:${arch}" in
    Linux:x86_64)
      echo "wasi-sdk-${WASI_SDK_VERSION}-x86_64-linux.tar.gz"
      ;;
    Linux:aarch64 | Linux:arm64)
      echo "wasi-sdk-${WASI_SDK_VERSION}-arm64-linux.tar.gz"
      ;;
    Darwin:x86_64)
      echo "wasi-sdk-${WASI_SDK_VERSION}-x86_64-macos.tar.gz"
      ;;
    Darwin:arm64)
      echo "wasi-sdk-${WASI_SDK_VERSION}-arm64-macos.tar.gz"
      ;;
    *)
      echo ""
      ;;
  esac
}

ensure_wasi_sdk() {
  local archive_name archive_path default_dir download_url

  if [ -n "${WASI_SDK:-}" ] && [ -x "${WASI_SDK}/bin/clang" ]; then
    echo "${WASI_SDK}"
    return
  fi

  archive_name="$(host_archive_name)"
  if [ -z "${archive_name}" ]; then
    echo "Unsupported host for auto-downloaded wasi-sdk. Set WASI_SDK to an extracted wasi-sdk directory." >&2
    exit 1
  fi

  archive_path="${BUILD_DIR}/${archive_name}"
  default_dir="${BUILD_DIR}/${archive_name%.tar.gz}"
  download_url="https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_SDK_TAG}/${archive_name}"

  mkdir -p "${BUILD_DIR}"

  if [ ! -f "${archive_path}" ]; then
    echo "==> Downloading ${archive_name}..." >&2
    curl -fL -H "User-Agent: codex" -o "${archive_path}" "${download_url}"
  fi

  if [ ! -x "${default_dir}/bin/clang" ]; then
    echo "==> Extracting ${archive_name}..." >&2
    tar -xzf "${archive_path}" -C "${BUILD_DIR}"
  fi

  echo "${default_dir}"
}

ensure_quickjs_source() {
  local archive_name archive_path source_dir source_url

  archive_name="quickjs-${QUICKJS_ARCHIVE_VERSION}.tar.xz"
  archive_path="${BUILD_DIR}/${archive_name}"
  source_dir="${BUILD_DIR}/quickjs-${QUICKJS_SOURCE_VERSION}"
  source_url="https://bellard.org/quickjs/${archive_name}"

  mkdir -p "${BUILD_DIR}"

  if [ ! -f "${archive_path}" ]; then
    echo "==> Downloading ${archive_name}..." >&2
    curl -fL -o "${archive_path}" "${source_url}"
  fi

  if [ ! -d "${source_dir}" ]; then
    echo "==> Extracting ${archive_name}..." >&2
    tar -xJf "${archive_path}" -C "${BUILD_DIR}"
  fi

  echo "${source_dir}"
}

main() {
  local wasi_sdk_dir quickjs_dir

  wasi_sdk_dir="$(ensure_wasi_sdk)"
  quickjs_dir="$(ensure_quickjs_source)"

  if [ ! -f "${WRAPPER_C}" ]; then
    echo "Missing QuickJS wrapper source: ${WRAPPER_C}" >&2
    exit 1
  fi

  echo "==> Building real QuickJS WASI module..."
  "${wasi_sdk_dir}/bin/clang" \
    --target=wasm32-wasip1 \
    --sysroot="${wasi_sdk_dir}/share/wasi-sysroot" \
    -O2 \
    -mllvm -wasm-enable-sjlj \
    -I"${quickjs_dir}" \
    -D_GNU_SOURCE \
    -DEMSCRIPTEN \
    -DCONFIG_VERSION="\"${QUICKJS_SOURCE_VERSION}\"" \
    "${quickjs_dir}/quickjs.c" \
    "${quickjs_dir}/dtoa.c" \
    "${quickjs_dir}/libregexp.c" \
    "${quickjs_dir}/libunicode.c" \
    "${quickjs_dir}/cutils.c" \
    "${WRAPPER_C}" \
    -o "${OUT_WASM}"

  ls -lh "${OUT_WASM}"
}

main "$@"
