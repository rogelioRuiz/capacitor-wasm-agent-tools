mod engine;
mod js_runtime;
mod python_runtime;
mod sandbox;
mod wasi_executor;

pub use js_runtime::{JsConfig, JsResult, JsRuntime};
pub use python_runtime::{PythonConfig, PythonResult, PythonRuntime};
pub use sandbox::{SandboxConfig, SandboxResult, WasmSandbox};

pub(crate) use engine::{
    build_store_limits, create_abi_store, create_wasi_store, effective_fuel_budget, elapsed_ms,
    integrity_error, io_error, map_execution_error, output_pipe_capacity, AbiStoreState,
    ExecutionConfig, ExecutionResult, SharedEngineHost, Watchdog,
};
pub(crate) use wasi_executor::execute_wasi;

uniffi::setup_scaffolding!();

#[derive(uniffi::Error, Debug, Clone)]
pub enum WasmToolsError {
    Io { msg: String },
    Execution { msg: String },
    IntegrityViolation { msg: String },
}

impl std::fmt::Display for WasmToolsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmToolsError::Io { msg } => write!(f, "Io: {msg}"),
            WasmToolsError::Execution { msg } => write!(f, "Execution: {msg}"),
            WasmToolsError::IntegrityViolation { msg } => write!(f, "IntegrityViolation: {msg}"),
        }
    }
}
