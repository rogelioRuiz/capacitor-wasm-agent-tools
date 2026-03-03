use std::fs;
use wasmtime::{Config, Engine};

fn main() {
    let targets = [
        ("aarch64-linux-android", "android-arm64"),
        ("aarch64-apple-ios", "ios-arm64"),
    ];

    let wasm_files = [
        ("wasm/rustpython.wasm", "rustpython"),
        ("wasm/quickjs.wasm", "quickjs"),
    ];

    for (target_triple, suffix) in &targets {
        let mut config = Config::new();
        // Must match create_engine() in engine.rs exactly
        config.consume_fuel(true);
        config.epoch_interruption(true);
        config
            .target(target_triple)
            .unwrap_or_else(|e| panic!("failed to set target {target_triple}: {e}"));

        let engine = Engine::new(&config)
            .unwrap_or_else(|e| panic!("failed to create engine for {target_triple}: {e}"));

        for (wasm_path, name) in &wasm_files {
            let wasm_bytes = fs::read(wasm_path)
                .unwrap_or_else(|e| panic!("failed to read {wasm_path}: {e}"));

            eprintln!("precompiling {name} for {target_triple} ({} bytes)...", wasm_bytes.len());

            let precompiled = engine
                .precompile_module(&wasm_bytes)
                .unwrap_or_else(|e| panic!("failed to precompile {name} for {target_triple}: {e}"));

            let out_path = format!("wasm/{name}-{suffix}.cwasm");
            fs::write(&out_path, &precompiled)
                .unwrap_or_else(|e| panic!("failed to write {out_path}: {e}"));

            eprintln!("  -> {out_path}: {} bytes", precompiled.len());
        }
    }

    eprintln!("done");
}
