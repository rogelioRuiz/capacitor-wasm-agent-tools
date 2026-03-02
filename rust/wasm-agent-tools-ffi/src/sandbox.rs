use crate::{
    build_store_limits, create_abi_store, effective_fuel_budget, elapsed_ms, io_error,
    map_execution_error, AbiStoreState, ExecutionConfig, SharedEngineHost, WasmToolsError,
    Watchdog,
};
use std::sync::Arc;
use std::time::Instant;
use wasmtime::{Linker, Memory, Module, Store};

const DEFAULT_FUEL_LIMIT: u64 = 1_000_000;
const DEFAULT_MAX_MEMORY_BYTES: u64 = 16 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[cfg(test)]
const ECHO_WAT: &str = r#"(module
    (memory (export "memory") 1)
    (global $bump (mut i32) (i32.const 1024))
    (func (export "alloc") (param $size i32) (result i32)
        (local $ptr i32)
        (local.set $ptr (global.get $bump))
        (global.set $bump (i32.add (global.get $bump) (local.get $size)))
        (local.get $ptr))
    (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
        (i64.or
            (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
            (i64.extend_i32_u (local.get $len)))))"#;

#[cfg(test)]
const INFINITE_LOOP_WAT: &str = r#"(module
    (memory (export "memory") 1)
    (global $bump (mut i32) (i32.const 1024))
    (func (export "alloc") (param $size i32) (result i32)
        (local $ptr i32)
        (local.set $ptr (global.get $bump))
        (global.set $bump (i32.add (global.get $bump) (local.get $size)))
        (local.get $ptr))
    (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
        (loop $inf (br $inf))
        (i64.const 0)))"#;

#[derive(uniffi::Record, Debug, Clone)]
pub struct SandboxConfig {
    pub fuel_limit: u64,
    pub max_memory_bytes: u64,
    pub timeout_secs: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            fuel_limit: DEFAULT_FUEL_LIMIT,
            max_memory_bytes: DEFAULT_MAX_MEMORY_BYTES,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl From<SandboxConfig> for ExecutionConfig {
    fn from(value: SandboxConfig) -> Self {
        Self {
            fuel_limit: value.fuel_limit,
            max_memory_bytes: value.max_memory_bytes,
            timeout_secs: value.timeout_secs,
        }
    }
}

#[derive(uniffi::Record, Debug, Clone)]
pub struct SandboxResult {
    pub output: String,
    pub fuel_consumed: u64,
    pub duration_ms: u64,
}

#[derive(uniffi::Object)]
pub struct WasmSandbox {
    host: Arc<SharedEngineHost>,
}

#[uniffi::export]
impl WasmSandbox {
    #[uniffi::constructor]
    pub fn create() -> Result<Arc<Self>, WasmToolsError> {
        Ok(Arc::new(Self {
            host: SharedEngineHost::global()?,
        }))
    }

    pub fn execute(
        &self,
        wasm_bytes: Vec<u8>,
        input_json: String,
        config: SandboxConfig,
    ) -> Result<SandboxResult, WasmToolsError> {
        let _execution_guard = self.host.lock();
        let started_at = Instant::now();
        let module = Module::new(self.host.engine(), &wasm_bytes)
            .map_err(|err| io_error(format!("WASM compilation failed: {err}")))?;

        let execution_config: ExecutionConfig = config.into();
        let store_limits = build_store_limits(execution_config.max_memory_bytes)?;
        let mut store = create_abi_store(self.host.engine(), store_limits);

        let fuel_budget = effective_fuel_budget(execution_config.fuel_limit);
        store
            .set_fuel(fuel_budget)
            .map_err(|err| io_error(format!("Failed to set sandbox fuel budget: {err}")))?;
        store.set_epoch_deadline(1);

        let _watchdog = Watchdog::spawn(self.host.engine().clone(), execution_config.timeout_secs);

        let linker = Linker::new(self.host.engine());
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|err| io_error(format!("WASM instantiation failed: {err}")))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| abi_error("missing export `memory`"))?;
        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|err| abi_error(format!("invalid `alloc` export: {err}")))?;
        let execute = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "execute")
            .map_err(|err| abi_error(format!("invalid `execute` export: {err}")))?;

        let input_bytes = input_json.into_bytes();
        let input_len = i32::try_from(input_bytes.len())
            .map_err(|_| abi_error("input exceeds guest ABI i32 length limit"))?;
        let input_ptr = alloc
            .call(&mut store, input_len)
            .map_err(map_execution_error)?;
        let input_offset = validate_offset(input_ptr, input_bytes.len(), "input allocation")?;
        ensure_memory_range(
            &memory,
            &store,
            input_offset,
            input_bytes.len(),
            "input write",
        )?;
        memory
            .write(&mut store, input_offset, &input_bytes)
            .map_err(|err| abi_error(format!("input write failed: {err}")))?;

        let packed_result = execute
            .call(&mut store, (input_ptr, input_len))
            .map_err(map_execution_error)?;
        let (result_offset, result_len) = unpack_result(packed_result)?;
        ensure_memory_range(&memory, &store, result_offset, result_len, "output read")?;

        let mut output_bytes = vec![0_u8; result_len];
        memory
            .read(&store, result_offset, &mut output_bytes)
            .map_err(|err| abi_error(format!("output read failed: {err}")))?;
        let output = String::from_utf8(output_bytes)
            .map_err(|err| abi_error(format!("output is not valid UTF-8 JSON: {err}")))?;

        let fuel_remaining = store
            .get_fuel()
            .map_err(|err| io_error(format!("Failed to read remaining fuel: {err}")))?;

        Ok(SandboxResult {
            output,
            fuel_consumed: fuel_budget.saturating_sub(fuel_remaining),
            duration_ms: elapsed_ms(started_at),
        })
    }

    pub fn execute_wat(
        &self,
        wat_text: String,
        input_json: String,
        config: SandboxConfig,
    ) -> Result<SandboxResult, WasmToolsError> {
        self.execute(wat_text.into_bytes(), input_json, config)
    }
}

fn unpack_result(packed_result: i64) -> Result<(usize, usize), WasmToolsError> {
    let packed_bits = packed_result as u64;
    let result_ptr = ((packed_bits >> 32) as u32) as usize;
    let result_len = (packed_bits as u32) as usize;
    Ok((result_ptr, result_len))
}

fn validate_offset(ptr: i32, len: usize, context: &str) -> Result<usize, WasmToolsError> {
    if ptr < 0 {
        return Err(abi_error(format!("{context} returned a negative pointer")));
    }

    let offset =
        usize::try_from(ptr).map_err(|_| abi_error(format!("{context} pointer overflow")))?;
    let _ = offset
        .checked_add(len)
        .ok_or_else(|| abi_error(format!("{context} range overflow")))?;
    Ok(offset)
}

fn ensure_memory_range(
    memory: &Memory,
    store: &Store<AbiStoreState>,
    offset: usize,
    len: usize,
    context: &str,
) -> Result<(), WasmToolsError> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| abi_error(format!("{context} range overflow")))?;
    let memory_size = memory.data_size(store);
    if end > memory_size {
        return Err(abi_error(format!(
            "{context} out of bounds: range {offset}..{end}, memory size {memory_size}"
        )));
    }
    Ok(())
}

fn abi_error(msg: impl Into<String>) -> WasmToolsError {
    WasmToolsError::IntegrityViolation {
        msg: format!("Guest ABI error: {}", msg.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox() -> Arc<WasmSandbox> {
        WasmSandbox::create().expect("sandbox should initialize")
    }

    fn default_config() -> SandboxConfig {
        SandboxConfig::default()
    }

    fn expect_io_error(err: WasmToolsError, needle: &str) {
        match err {
            WasmToolsError::Io { msg } => assert!(
                msg.contains(needle),
                "expected `{needle}` in io error, got `{msg}`"
            ),
            other => panic!("expected io error, got {other:?}"),
        }
    }

    fn expect_integrity_error(err: WasmToolsError, needle: &str) {
        match err {
            WasmToolsError::IntegrityViolation { msg } => assert!(
                msg.contains(needle),
                "expected `{needle}` in integrity error, got `{msg}`"
            ),
            other => panic!("expected integrity error, got {other:?}"),
        }
    }

    #[test]
    fn test_sandbox_create() {
        let sandbox = WasmSandbox::create();
        assert!(sandbox.is_ok());
    }

    #[test]
    fn test_echo_module() {
        let sandbox = sandbox();
        let input = r#"{"echo":"ok"}"#.to_string();

        let result = sandbox
            .execute(
                ECHO_WAT.as_bytes().to_vec(),
                input.clone(),
                default_config(),
            )
            .expect("echo module should succeed");

        assert_eq!(result.output, input);
    }

    #[test]
    fn test_fuel_consumed() {
        let sandbox = sandbox();
        let result = sandbox
            .execute_wat(ECHO_WAT.to_string(), "{}".to_string(), default_config())
            .expect("echo module should succeed");

        assert!(result.fuel_consumed > 0, "expected fuel to be consumed");
    }

    #[test]
    fn test_fuel_exhaustion() {
        let sandbox = sandbox();
        let config = SandboxConfig {
            fuel_limit: 10_000,
            ..default_config()
        };

        let err = sandbox
            .execute_wat(INFINITE_LOOP_WAT.to_string(), "{}".to_string(), config)
            .expect_err("infinite loop should exhaust fuel");

        expect_integrity_error(err, "Fuel exhausted");
    }

    #[test]
    fn test_timeout() {
        let sandbox = sandbox();
        let config = SandboxConfig {
            fuel_limit: 0,
            timeout_secs: 2,
            ..default_config()
        };

        let err = sandbox
            .execute_wat(INFINITE_LOOP_WAT.to_string(), "{}".to_string(), config)
            .expect_err("infinite loop should time out");

        expect_integrity_error(err, "Timeout");
    }

    #[test]
    fn test_invalid_wasm() {
        let sandbox = sandbox();
        let err = sandbox
            .execute(vec![0, 1, 2, 3, 4], "{}".to_string(), default_config())
            .expect_err("random bytes should fail to compile");

        expect_io_error(err, "WASM compilation failed");
    }

    #[test]
    fn test_missing_exports() {
        let sandbox = sandbox();
        let missing_execute = r#"(module
            (memory (export "memory") 1)
            (func (export "alloc") (param $size i32) (result i32)
                (i32.const 0)))"#;

        let err = sandbox
            .execute_wat(
                missing_execute.to_string(),
                "{}".to_string(),
                default_config(),
            )
            .expect_err("missing execute export should fail");

        expect_integrity_error(err, "ABI error");
    }

    #[test]
    fn test_execute_wat() {
        let sandbox = sandbox();
        let input = r#"{"wat":true}"#.to_string();

        let result = sandbox
            .execute_wat(ECHO_WAT.to_string(), input.clone(), default_config())
            .expect("execute_wat should succeed");

        assert_eq!(result.output, input);
    }

    #[test]
    fn test_memory_limit() {
        let sandbox = sandbox();
        let oversized = r#"(module
            (memory (export "memory") 2)
            (func (export "alloc") (param $size i32) (result i32)
                (i32.const 0))
            (func (export "execute") (param $ptr i32) (param $len i32) (result i64)
                (i64.const 0)))"#;
        let config = SandboxConfig {
            max_memory_bytes: 64 * 1024,
            ..default_config()
        };

        let err = sandbox
            .execute_wat(oversized.to_string(), "{}".to_string(), config)
            .expect_err("oversized memory should fail");

        expect_io_error(err, "WASM instantiation failed");
    }
}
