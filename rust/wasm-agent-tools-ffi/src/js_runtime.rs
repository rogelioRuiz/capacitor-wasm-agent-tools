use crate::{
    execute_wasi, integrity_error, io_error, ExecutionConfig, SharedEngineHost, WasmToolsError,
};
use std::sync::Arc;
use wasmtime::Module;

const DEFAULT_FUEL_LIMIT: u64 = 1_000_000_000;
const DEFAULT_MAX_MEMORY_BYTES: u64 = 32 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 5;
const QUICKJS_WASM_BYTES: &[u8] = include_bytes!("../wasm/quickjs.wasm");

#[derive(uniffi::Record, Debug, Clone)]
pub struct JsConfig {
    pub timeout_secs: u64,
    pub fuel_limit: u64,
    pub max_memory_bytes: u64,
}

impl Default for JsConfig {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            fuel_limit: DEFAULT_FUEL_LIMIT,
            max_memory_bytes: DEFAULT_MAX_MEMORY_BYTES,
        }
    }
}

impl From<JsConfig> for ExecutionConfig {
    fn from(value: JsConfig) -> Self {
        Self {
            fuel_limit: value.fuel_limit,
            max_memory_bytes: value.max_memory_bytes,
            timeout_secs: value.timeout_secs,
        }
    }
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct JsResult {
    pub output: String,
    pub error: Option<String>,
    pub fuel_consumed: u64,
    pub duration_ms: u64,
}

#[derive(uniffi::Object)]
pub struct JsRuntime {
    host: Arc<SharedEngineHost>,
    quickjs_module: Module,
}

#[uniffi::export]
impl JsRuntime {
    #[uniffi::constructor]
    pub fn create() -> Result<Arc<Self>, WasmToolsError> {
        let host = SharedEngineHost::global()?;
        let quickjs_module = compile_embedded_module(host.engine(), "QuickJS", QUICKJS_WASM_BYTES)?;
        Ok(Arc::new(Self {
            host,
            quickjs_module,
        }))
    }

    pub fn execute(&self, code: String, config: JsConfig) -> Result<JsResult, WasmToolsError> {
        let args = ["quickjs", "-e", code.as_str()];
        let execution = execute_wasi(
            &self.host,
            &self.quickjs_module,
            &args,
            None,
            &config.into(),
        )?;

        let error = if execution.exit_code == 0 {
            None
        } else if execution.stderr.trim().is_empty() {
            Some(format!(
                "QuickJS exited with status {}",
                execution.exit_code
            ))
        } else {
            Some(execution.stderr.trim().to_string())
        };

        Ok(JsResult {
            output: execution.stdout,
            error,
            fuel_consumed: execution.fuel_consumed,
            duration_ms: execution.duration_ms,
        })
    }
}

fn compile_embedded_module(
    engine: &wasmtime::Engine,
    name: &str,
    bytes: &[u8],
) -> Result<Module, WasmToolsError> {
    if !bytes.starts_with(b"\0asm") {
        return Err(io_error(format!(
            "{name} WASM artifact is missing or still a placeholder; replace rust/wasm-agent-tools-ffi/wasm/quickjs.wasm with a real wasm32-wasip1 binary"
        )));
    }

    Module::new(engine, bytes)
        .map_err(|err| integrity_error(format!("Failed to compile embedded {name} module: {err}")))
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
                let fixture_dir = manifest_dir.join("tests/fixtures/quickjs-cli");
                let fixture_target = manifest_dir.join("target/test-quickjs-cli");
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
                    .expect("failed to invoke cargo for quickjs fixture");
                assert!(status.success(), "quickjs fixture build should succeed");
                fixture_target.join("wasm32-wasip1/release/quickjs-cli.wasm")
            })
            .clone()
    }

    fn fixture_runtime() -> Arc<JsRuntime> {
        let host = SharedEngineHost::global().expect("host should initialize");
        let quickjs_module =
            Module::from_file(host.engine(), fixture_wasm_path()).expect("fixture module should load");

        Arc::new(JsRuntime {
            host,
            quickjs_module,
        })
    }

    #[test]
    fn executes_console_log() {
        let runtime = fixture_runtime();
        let result = runtime
            .execute("console.log(1 + 1)".to_string(), JsConfig::default())
            .expect("quickjs execution should succeed");

        assert_eq!(result.output, "2\n");
        assert_eq!(result.error, None);
    }

    #[test]
    fn returns_expression_value() {
        let runtime = fixture_runtime();
        let result = runtime
            .execute("1 + 1".to_string(), JsConfig::default())
            .expect("quickjs execution should succeed");

        assert_eq!(result.output, "2\n");
        assert_eq!(result.error, None);
    }

    #[test]
    fn surfaces_throw_errors() {
        let runtime = fixture_runtime();
        let result = runtime
            .execute("throw new Error('test')".to_string(), JsConfig::default())
            .expect("quickjs execution should succeed");

        assert_eq!(result.output, "");
        let error = result.error.expect("expected quickjs error output");
        assert!(error.contains("test"), "unexpected error text: {error}");
    }
}
