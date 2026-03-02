import Capacitor
import Foundation

private let defaultSandboxFuelLimit: UInt64 = 1_000_000
private let defaultSandboxMaxMemoryBytes: UInt64 = 16 * 1024 * 1024
private let defaultSandboxTimeoutSecs: UInt64 = 30
private let defaultJsFuelLimit: UInt64 = 1_000_000_000
private let defaultJsMaxMemoryBytes: UInt64 = 32 * 1024 * 1024
private let defaultJsTimeoutSecs: UInt64 = 5
private let defaultPythonFuelLimit: UInt64 = 1_000_000_000
private let defaultPythonMaxMemoryBytes: UInt64 = 32 * 1024 * 1024
private let defaultPythonTimeoutSecs: UInt64 = 5

@objc(WasmAgentToolsPlugin)
public class WasmAgentToolsPlugin: CAPPlugin, CAPBridgedPlugin {
    public let identifier = "WasmAgentToolsPlugin"
    public let jsName = "WasmAgentTools"
    public let pluginMethods: [CAPPluginMethod] = [
        CAPPluginMethod(name: "createWasmSandbox", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "executeSandbox", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "executeWatSandbox", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "createJsRuntime", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "executeJs", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "createPythonRuntime", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "executePython", returnType: CAPPluginReturnPromise),
    ]

    private var wasmSandbox: WasmSandbox?
    private var jsRuntime: JsRuntime?
    private var pythonRuntime: PythonRuntime?

    @objc func createWasmSandbox(_ call: CAPPluginCall) {
        do {
            wasmSandbox = nil
            wasmSandbox = try WasmSandbox.create()
            call.resolve()
        } catch {
            call.reject("createWasmSandbox failed: \(error.localizedDescription)")
        }
    }

    @objc func executeSandbox(_ call: CAPPluginCall) {
        guard let wasmSandbox = requireWasmSandbox(call) else { return }
        guard let wasmBase64 = call.getString("wasmBase64") else {
            return call.reject("wasmBase64 is required")
        }
        guard let inputJson = call.getString("inputJson") else {
            return call.reject("inputJson is required")
        }
        guard let wasmBytes = Data(base64Encoded: wasmBase64) else {
            return call.reject("Invalid base64")
        }

        do {
            let result = try wasmSandbox.execute(
                wasmBytes: wasmBytes,
                inputJson: inputJson,
                config: sandboxConfig(from: call.getObject("config"))
            )
            call.resolve(sandboxResultToDictionary(result))
        } catch {
            call.reject("executeSandbox failed: \(error.localizedDescription)")
        }
    }

    @objc func executeWatSandbox(_ call: CAPPluginCall) {
        guard let wasmSandbox = requireWasmSandbox(call) else { return }
        guard let watText = call.getString("watText") else {
            return call.reject("watText is required")
        }
        guard let inputJson = call.getString("inputJson") else {
            return call.reject("inputJson is required")
        }

        do {
            let result = try wasmSandbox.executeWat(
                watText: watText,
                inputJson: inputJson,
                config: sandboxConfig(from: call.getObject("config"))
            )
            call.resolve(sandboxResultToDictionary(result))
        } catch {
            call.reject("executeWatSandbox failed: \(error.localizedDescription)")
        }
    }

    @objc func createJsRuntime(_ call: CAPPluginCall) {
        do {
            jsRuntime = nil
            jsRuntime = try JsRuntime.create()
            call.resolve()
        } catch {
            call.reject("createJsRuntime failed: \(error.localizedDescription)")
        }
    }

    @objc func executeJs(_ call: CAPPluginCall) {
        guard let jsRuntime = requireJsRuntime(call) else { return }
        guard let code = call.getString("code") else {
            return call.reject("code is required")
        }

        do {
            let result = try jsRuntime.execute(
                code: code,
                config: jsConfig(from: call.getObject("config"))
            )
            call.resolve(jsResultToDictionary(result))
        } catch {
            call.reject("executeJs failed: \(error.localizedDescription)")
        }
    }

    @objc func createPythonRuntime(_ call: CAPPluginCall) {
        do {
            pythonRuntime = nil
            pythonRuntime = try PythonRuntime.create()
            call.resolve()
        } catch {
            call.reject("createPythonRuntime failed: \(error.localizedDescription)")
        }
    }

    @objc func executePython(_ call: CAPPluginCall) {
        guard let pythonRuntime = requirePythonRuntime(call) else { return }
        guard let code = call.getString("code") else {
            return call.reject("code is required")
        }

        do {
            let result = try pythonRuntime.execute(
                code: code,
                config: pythonConfig(from: call.getObject("config"))
            )
            call.resolve(pythonResultToDictionary(result))
        } catch {
            call.reject("executePython failed: \(error.localizedDescription)")
        }
    }

    private func requireWasmSandbox(_ call: CAPPluginCall) -> WasmSandbox? {
        guard let wasmSandbox else {
            call.reject("WasmSandbox not initialized — call createWasmSandbox() first")
            return nil
        }
        return wasmSandbox
    }

    private func requireJsRuntime(_ call: CAPPluginCall) -> JsRuntime? {
        guard let jsRuntime else {
            call.reject("JsRuntime not initialized - call createJsRuntime() first")
            return nil
        }
        return jsRuntime
    }

    private func requirePythonRuntime(_ call: CAPPluginCall) -> PythonRuntime? {
        guard let pythonRuntime else {
            call.reject("PythonRuntime not initialized - call createPythonRuntime() first")
            return nil
        }
        return pythonRuntime
    }

    private func uint64(_ value: JSValue?) -> UInt64? {
        if let n = value as? NSNumber { return n.uint64Value }
        return nil
    }

    private func sandboxConfig(from config: JSObject?) -> SandboxConfig {
        let fuelLimit = uint64(config?["fuelLimit"]) ?? defaultSandboxFuelLimit
        let maxMemoryBytes = uint64(config?["maxMemoryBytes"]) ?? defaultSandboxMaxMemoryBytes
        let timeoutSecs = uint64(config?["timeoutSecs"]) ?? defaultSandboxTimeoutSecs
        return SandboxConfig(
            fuelLimit: fuelLimit,
            maxMemoryBytes: maxMemoryBytes,
            timeoutSecs: timeoutSecs
        )
    }

    private func jsConfig(from config: JSObject?) -> JsConfig {
        let fuelLimit = uint64(config?["fuelLimit"]) ?? defaultJsFuelLimit
        let maxMemoryBytes = uint64(config?["maxMemoryBytes"]) ?? defaultJsMaxMemoryBytes
        let timeoutSecs = uint64(config?["timeoutSecs"]) ?? defaultJsTimeoutSecs
        return JsConfig(
            timeoutSecs: timeoutSecs,
            fuelLimit: fuelLimit,
            maxMemoryBytes: maxMemoryBytes
        )
    }

    private func pythonConfig(from config: JSObject?) -> PythonConfig {
        let fuelLimit = uint64(config?["fuelLimit"]) ?? defaultPythonFuelLimit
        let maxMemoryBytes = uint64(config?["maxMemoryBytes"]) ?? defaultPythonMaxMemoryBytes
        let timeoutSecs = uint64(config?["timeoutSecs"]) ?? defaultPythonTimeoutSecs
        return PythonConfig(
            timeoutSecs: timeoutSecs,
            fuelLimit: fuelLimit,
            maxMemoryBytes: maxMemoryBytes
        )
    }

    private func sandboxResultToDictionary(_ result: SandboxResult) -> [String: Any] {
        [
            "output": result.output,
            "fuelConsumed": result.fuelConsumed,
            "durationMs": result.durationMs,
        ]
    }

    private func jsResultToDictionary(_ result: JsResult) -> [String: Any] {
        var payload: [String: Any] = [
            "output": result.output,
            "fuelConsumed": result.fuelConsumed,
            "durationMs": result.durationMs,
        ]
        if let error = result.error {
            payload["error"] = error
        }
        return payload
    }

    private func pythonResultToDictionary(_ result: PythonResult) -> [String: Any] {
        var payload: [String: Any] = [
            "output": result.output,
            "fuelConsumed": result.fuelConsumed,
            "durationMs": result.durationMs,
        ]
        if let error = result.error {
            payload["error"] = error
        }
        return payload
    }
}
