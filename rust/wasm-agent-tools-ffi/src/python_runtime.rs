use crate::{
    execute_wasi, integrity_error, io_error, ExecutionConfig, SharedEngineHost, WasmToolsError,
};
use std::sync::Arc;
use wasmtime::Module;

const DEFAULT_FUEL_LIMIT: u64 = 1_000_000_000;
const DEFAULT_MAX_MEMORY_BYTES: u64 = 32 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 5;
// Raw WASM kept as fallback for desktop/tests where precompiled isn't available
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const RUSTPYTHON_WASM_BYTES: &[u8] = include_bytes!("../wasm/rustpython.wasm");

// Precompiled Cranelift machine code — produced by `cargo run --bin precompile`
#[cfg(target_os = "android")]
const RUSTPYTHON_PRECOMPILED: &[u8] = include_bytes!("../wasm/rustpython-android-arm64.cwasm");
#[cfg(target_os = "ios")]
const RUSTPYTHON_PRECOMPILED: &[u8] = include_bytes!("../wasm/rustpython-ios-arm64.cwasm");
const PYTHON_PRELUDE: &str = r#"import sys
class _Blocked:
    def __getattr__(self, name):
        raise ImportError("module blocked in sandbox")
for m in ['subprocess', 'socket', 'ctypes']:
    sys.modules[m] = _Blocked()
_code = sys.stdin.read()
_g = {'__builtins__': __builtins__, '__name__': '__main__'}
exec(_code, _g, _g)
"#;

#[derive(uniffi::Record, Debug, Clone)]
pub struct PythonConfig {
    pub timeout_secs: u64,
    pub fuel_limit: u64,
    pub max_memory_bytes: u64,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            fuel_limit: DEFAULT_FUEL_LIMIT,
            max_memory_bytes: DEFAULT_MAX_MEMORY_BYTES,
        }
    }
}

impl From<PythonConfig> for ExecutionConfig {
    fn from(value: PythonConfig) -> Self {
        Self {
            fuel_limit: value.fuel_limit,
            max_memory_bytes: value.max_memory_bytes,
            timeout_secs: value.timeout_secs,
        }
    }
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct PythonResult {
    pub output: String,
    pub error: Option<String>,
    pub fuel_consumed: u64,
    pub duration_ms: u64,
}

#[derive(uniffi::Object)]
pub struct PythonRuntime {
    host: Arc<SharedEngineHost>,
    rustpython_module: Module,
}

#[uniffi::export]
impl PythonRuntime {
    #[uniffi::constructor]
    pub fn create() -> Result<Arc<Self>, WasmToolsError> {
        let host = SharedEngineHost::global()?;
        let rustpython_module = load_rustpython_module(host.engine())?;
        Ok(Arc::new(Self {
            host,
            rustpython_module,
        }))
    }

    pub fn execute(
        &self,
        code: String,
        config: PythonConfig,
    ) -> Result<PythonResult, WasmToolsError> {
        let execution = execute_wasi(
            &self.host,
            &self.rustpython_module,
            &["rustpython", "-c", PYTHON_PRELUDE],
            Some(code.as_bytes()),
            &config.into(),
        )?;

        let error = if execution.exit_code == 0 {
            None
        } else if execution.stderr.trim().is_empty() {
            Some(format!(
                "RustPython exited with status {}",
                execution.exit_code
            ))
        } else {
            Some(execution.stderr.trim().to_string())
        };

        Ok(PythonResult {
            output: execution.stdout,
            error,
            fuel_consumed: execution.fuel_consumed,
            duration_ms: execution.duration_ms,
        })
    }
}

/// On mobile (Android/iOS), load the precompiled Cranelift machine code.
/// On desktop/tests, JIT-compile from raw WASM bytes.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn load_rustpython_module(engine: &wasmtime::Engine) -> Result<Module, WasmToolsError> {
    // SAFETY: the precompiled bytes were produced by the same wasmtime version
    // and engine config (consume_fuel + epoch_interruption) via `cargo run --bin precompile`.
    unsafe { Module::deserialize(engine, RUSTPYTHON_PRECOMPILED) }
        .map_err(|err| integrity_error(format!("Failed to load precompiled RustPython module: {err}")))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn load_rustpython_module(engine: &wasmtime::Engine) -> Result<Module, WasmToolsError> {
    if !RUSTPYTHON_WASM_BYTES.starts_with(b"\0asm") {
        return Err(io_error(
            "RustPython WASM artifact is missing or still a placeholder; replace rust/wasm-agent-tools-ffi/wasm/rustpython.wasm with a real wasm32-wasip1 binary"
        ));
    }
    Module::new(engine, RUSTPYTHON_WASM_BYTES)
        .map_err(|err| integrity_error(format!("Failed to compile RustPython module: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::OnceLock;

    static FIXTURE_WASM: OnceLock<PathBuf> = OnceLock::new();

    fn fixture_wasm_path() -> PathBuf {
        FIXTURE_WASM
            .get_or_init(|| {
                let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                let fixture_dir = manifest_dir.join("tests/fixtures/python-cli");
                let fixture_target = manifest_dir.join("target/test-python-cli");
                let status = Command::new("cargo")
                    .arg("build")
                    .arg("--release")
                    .arg("--target")
                    .arg("wasm32-wasip1")
                    .arg("--target-dir")
                    .arg(&fixture_target)
                    .arg("--manifest-path")
                    .arg(fixture_dir.join("Cargo.toml"))
                    .status()
                    .expect("failed to invoke cargo for python fixture");
                assert!(status.success(), "python fixture build should succeed");
                fixture_target.join("wasm32-wasip1/release/python-cli.wasm")
            })
            .clone()
    }

    fn fixture_runtime() -> Arc<PythonRuntime> {
        let host = SharedEngineHost::global().expect("host should initialize");
        let rustpython_module =
            Module::from_file(host.engine(), fixture_wasm_path()).expect("fixture module should load");

        Arc::new(PythonRuntime {
            host,
            rustpython_module,
        })
    }

    #[test]
    fn executes_simple_print() {
        let runtime = fixture_runtime();
        let result = runtime
            .execute("print(1 + 1)".to_string(), PythonConfig::default())
            .expect("python execution should succeed");

        assert_eq!(result.output, "2\n");
        assert_eq!(result.error, None);
    }
}
