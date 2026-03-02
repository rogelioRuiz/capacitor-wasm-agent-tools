use crate::{
    build_store_limits, create_wasi_store, effective_fuel_budget, elapsed_ms, io_error,
    map_execution_error, output_pipe_capacity, ExecutionConfig, ExecutionResult, SharedEngineHost,
    WasmToolsError, Watchdog,
};
use std::time::Instant;
use wasmtime::{Error, Linker, Module};
use wasmtime_wasi::{preview1, I32Exit};

pub fn execute_wasi(
    host: &SharedEngineHost,
    module: &Module,
    args: &[&str],
    stdin: Option<&[u8]>,
    config: &ExecutionConfig,
) -> Result<ExecutionResult, WasmToolsError> {
    let _execution_guard = host.lock();
    let started_at = Instant::now();
    let store_limits = build_store_limits(config.max_memory_bytes)?;
    let output_capacity = output_pipe_capacity(config);
    let (mut store, stdout, stderr) =
        create_wasi_store(host.engine(), store_limits, args, stdin, output_capacity)?;

    let fuel_budget = effective_fuel_budget(config.fuel_limit);
    store
        .set_fuel(fuel_budget)
        .map_err(|err| io_error(format!("Failed to set WASI fuel budget: {err}")))?;
    store.set_epoch_deadline(1);

    let _watchdog = Watchdog::spawn(host.engine().clone(), config.timeout_secs);

    let mut linker: Linker<crate::engine::WasiStoreState> = Linker::new(host.engine());
    preview1::add_to_linker_sync(&mut linker, |state: &mut crate::engine::WasiStoreState| {
        &mut state.wasi
    })
    .map_err(|err| io_error(format!("Failed to configure WASI linker: {err}")))?;

    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|err| io_error(format!("WASI instantiation failed: {err}")))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|err| io_error(format!("WASI module is missing `_start`: {err}")))?;

    let call_result: Result<(), Error> = start.call(&mut store, ());
    let exit_code = match call_result {
        Ok(()) => 0,
        Err(err) => match err.downcast_ref::<I32Exit>() {
            Some(exit) => exit.0,
            None => return Err(map_execution_error(err)),
        },
    };

    let fuel_remaining = store
        .get_fuel()
        .map_err(|err| io_error(format!("Failed to read remaining WASI fuel: {err}")))?;
    let stdout = String::from_utf8(stdout.contents().to_vec())
        .map_err(|err| io_error(format!("WASI stdout is not valid UTF-8: {err}")))?;
    let stderr = String::from_utf8(stderr.contents().to_vec())
        .map_err(|err| io_error(format!("WASI stderr is not valid UTF-8: {err}")))?;

    Ok(ExecutionResult {
        stdout,
        stderr,
        exit_code,
        fuel_consumed: fuel_budget.saturating_sub(fuel_remaining),
        duration_ms: elapsed_ms(started_at),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SharedEngineHost;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::OnceLock;

    static FIXTURE_WASM: OnceLock<PathBuf> = OnceLock::new();

    fn fixture_wasm_path() -> PathBuf {
        FIXTURE_WASM
            .get_or_init(|| {
                let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                let fixture_dir = manifest_dir.join("tests/fixtures/wasi-smoke");
                let fixture_target = manifest_dir.join("target/test-wasi-smoke");
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
                    .expect("failed to invoke cargo for wasi fixture");
                assert!(status.success(), "wasi fixture build should succeed");
                fixture_target.join("wasm32-wasip1/release/wasi-smoke.wasm")
            })
            .clone()
    }

    fn fixture_module() -> Module {
        let host = SharedEngineHost::global().expect("host should initialize");
        Module::from_file(host.engine(), fixture_wasm_path()).expect("fixture module should load")
    }

    fn default_config() -> ExecutionConfig {
        ExecutionConfig {
            fuel_limit: 1_000_000,
            max_memory_bytes: 16 * 1024 * 1024,
            timeout_secs: 5,
        }
    }

    #[test]
    fn test_wasi_stdout_stderr_and_stdin() {
        let host = SharedEngineHost::global().expect("host should initialize");
        let module = fixture_module();

        let result = execute_wasi(
            &host,
            &module,
            &["wasi-smoke", "echo"],
            Some(b"payload"),
            &default_config(),
        )
        .expect("wasi execution should succeed");

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("mode=echo"));
        assert!(result.stdout.contains("stdin=payload"));
        assert!(result.stderr.contains("stderr=echo"));
        assert!(result.fuel_consumed > 0);
    }

    #[test]
    fn test_wasi_nonzero_exit() {
        let host = SharedEngineHost::global().expect("host should initialize");
        let module = fixture_module();

        let result = execute_wasi(
            &host,
            &module,
            &["wasi-smoke", "exit7"],
            None,
            &default_config(),
        )
        .expect("wasi execution should return output even on nonzero exit");

        assert_eq!(result.exit_code, 7);
        assert!(result.stderr.contains("exit=7"));
    }

    #[test]
    fn test_wasi_timeout() {
        let host = SharedEngineHost::global().expect("host should initialize");
        let module = fixture_module();
        let config = ExecutionConfig {
            fuel_limit: 0,
            timeout_secs: 1,
            ..default_config()
        };

        let err = execute_wasi(&host, &module, &["wasi-smoke", "loop"], None, &config)
            .expect_err("infinite loop should time out");

        match err {
            WasmToolsError::IntegrityViolation { msg } => {
                assert!(msg.contains("Timeout"), "unexpected timeout message: {msg}");
            }
            other => panic!("expected timeout integrity error, got {other:?}"),
        }
    }
}
