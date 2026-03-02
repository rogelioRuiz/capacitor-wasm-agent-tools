use crate::WasmToolsError;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Instant;
use wasmtime::{Config, Engine, Error, Store, StoreLimits, StoreLimitsBuilder, Trap};
use wasmtime_wasi::{
    pipe::{MemoryInputPipe, MemoryOutputPipe},
    preview1::WasiP1Ctx,
    WasiCtxBuilder,
};

const UNLIMITED_FUEL_BUDGET: u64 = u64::MAX;
const DEFAULT_PIPE_CAPACITY: usize = 1024 * 1024;

static GLOBAL_HOST: OnceLock<Arc<SharedEngineHost>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub fuel_limit: u64,
    pub max_memory_bytes: u64,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub fuel_consumed: u64,
    pub duration_ms: u64,
}

pub struct SharedEngineHost {
    engine: Engine,
    execution_lock: Mutex<()>,
}

impl SharedEngineHost {
    pub fn global() -> Result<Arc<Self>, WasmToolsError> {
        if let Some(host) = GLOBAL_HOST.get() {
            return Ok(host.clone());
        }

        let candidate = Arc::new(Self::new()?);
        if GLOBAL_HOST.set(candidate.clone()).is_ok() {
            return Ok(candidate);
        }

        Ok(GLOBAL_HOST
            .get()
            .expect("global host should be initialized")
            .clone())
    }

    fn new() -> Result<Self, WasmToolsError> {
        Ok(Self {
            engine: create_engine()?,
            execution_lock: Mutex::new(()),
        })
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn lock(&self) -> MutexGuard<'_, ()> {
        self.execution_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub struct Watchdog {
    cancel_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl Watchdog {
    pub fn spawn(engine: Engine, timeout_secs: u64) -> Self {
        let (cancel_tx, cancel_rx) = std::sync::mpsc::channel();
        let timeout = std::time::Duration::from_secs(timeout_secs);
        std::thread::spawn(move || {
            if cancel_rx.recv_timeout(timeout).is_err() {
                engine.increment_epoch();
            }
        });
        Self {
            cancel_tx: Some(cancel_tx),
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
    }
}

pub struct AbiStoreState {
    pub limits: StoreLimits,
}

pub struct WasiStoreState {
    pub limits: StoreLimits,
    pub wasi: WasiP1Ctx,
}

pub fn create_engine() -> Result<Engine, WasmToolsError> {
    let mut config = Config::new();
    config.consume_fuel(true);
    config.epoch_interruption(true);

    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    config.macos_use_mach_ports(false);

    Engine::new(&config).map_err(|err| io_error(format!("Failed to create WASM engine: {err}")))
}

pub fn build_store_limits(max_memory_bytes: u64) -> Result<StoreLimits, WasmToolsError> {
    let max_memory_bytes = usize::try_from(max_memory_bytes).map_err(|_| {
        integrity_error("Sandbox config error: max_memory_bytes exceeds platform limit")
    })?;

    Ok(StoreLimitsBuilder::new()
        .memory_size(max_memory_bytes)
        .instances(1)
        .tables(1)
        .memories(1)
        .build())
}

pub fn create_abi_store(engine: &Engine, limits: StoreLimits) -> Store<AbiStoreState> {
    let mut store = Store::new(engine, AbiStoreState { limits });
    store.limiter(|state| &mut state.limits);
    store
}

pub fn create_wasi_store(
    engine: &Engine,
    limits: StoreLimits,
    args: &[&str],
    stdin: Option<&[u8]>,
    output_capacity: usize,
) -> Result<(Store<WasiStoreState>, MemoryOutputPipe, MemoryOutputPipe), WasmToolsError> {
    let stdout = MemoryOutputPipe::new(output_capacity);
    let stderr = MemoryOutputPipe::new(output_capacity);

    let mut builder = WasiCtxBuilder::new();
    builder.allow_blocking_current_thread(true);
    builder.args(args);
    builder.stdout(stdout.clone());
    builder.stderr(stderr.clone());
    if let Some(stdin) = stdin {
        builder.stdin(MemoryInputPipe::new(stdin.to_vec()));
    }

    let mut store = Store::new(
        engine,
        WasiStoreState {
            limits,
            wasi: builder.build_p1(),
        },
    );
    store.limiter(|state| &mut state.limits);

    Ok((store, stdout, stderr))
}

pub fn effective_fuel_budget(requested: u64) -> u64 {
    if requested == 0 {
        UNLIMITED_FUEL_BUDGET
    } else {
        requested
    }
}

pub fn output_pipe_capacity(config: &ExecutionConfig) -> usize {
    match usize::try_from(config.max_memory_bytes) {
        Ok(value) => value.max(DEFAULT_PIPE_CAPACITY),
        Err(_) => usize::MAX,
    }
}

pub fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub fn map_execution_error(err: Error) -> WasmToolsError {
    if let Some(trap) = err.downcast_ref::<Trap>() {
        return match trap {
            Trap::OutOfFuel => integrity_error("Fuel exhausted: exceeded CPU budget"),
            Trap::Interrupt => integrity_error("Timeout: exceeded wall-clock limit"),
            _ => output_error(format!("WASM execution failed: {err}")),
        };
    }

    output_error(format!("WASM execution failed: {err}"))
}

pub fn io_error(msg: impl Into<String>) -> WasmToolsError {
    WasmToolsError::Io { msg: msg.into() }
}

pub fn output_error(msg: impl Into<String>) -> WasmToolsError {
    WasmToolsError::Execution { msg: msg.into() }
}

pub fn integrity_error(msg: impl Into<String>) -> WasmToolsError {
    WasmToolsError::IntegrityViolation { msg: msg.into() }
}
