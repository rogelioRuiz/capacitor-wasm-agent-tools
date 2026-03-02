package com.t6x.plugins.wasmagenttools

import android.util.Base64
import com.getcapacitor.JSObject
import com.getcapacitor.Plugin
import com.getcapacitor.PluginCall
import com.getcapacitor.PluginMethod
import com.getcapacitor.annotation.CapacitorPlugin
import uniffi.wasm_agent_tools_ffi.JsConfig
import uniffi.wasm_agent_tools_ffi.JsResult
import uniffi.wasm_agent_tools_ffi.JsRuntime
import uniffi.wasm_agent_tools_ffi.PythonConfig
import uniffi.wasm_agent_tools_ffi.PythonResult
import uniffi.wasm_agent_tools_ffi.PythonRuntime
import uniffi.wasm_agent_tools_ffi.SandboxConfig
import uniffi.wasm_agent_tools_ffi.SandboxResult
import uniffi.wasm_agent_tools_ffi.WasmSandbox

private const val DEFAULT_FUEL_LIMIT = 1_000_000UL
private const val DEFAULT_MAX_MEMORY_BYTES = 16_777_216UL
private const val DEFAULT_TIMEOUT_SECS = 30UL
private const val DEFAULT_JS_FUEL_LIMIT = 1_000_000_000UL
private const val DEFAULT_JS_MAX_MEMORY_BYTES = 33_554_432UL
private const val DEFAULT_JS_TIMEOUT_SECS = 5UL
private const val DEFAULT_PYTHON_FUEL_LIMIT = 1_000_000_000UL
private const val DEFAULT_PYTHON_MAX_MEMORY_BYTES = 33_554_432UL
private const val DEFAULT_PYTHON_TIMEOUT_SECS = 5UL

@CapacitorPlugin(name = "WasmAgentTools")
class WasmAgentToolsPlugin : Plugin() {
    private var wasmSandbox: WasmSandbox? = null
    private var jsRuntime: JsRuntime? = null
    private var pythonRuntime: PythonRuntime? = null

    @PluginMethod
    fun createWasmSandbox(call: PluginCall) {
        try {
            wasmSandbox?.close()
            wasmSandbox = WasmSandbox.create()
            call.resolve()
        } catch (e: Exception) {
            call.reject("createWasmSandbox failed: ${e.message}", e)
        }
    }

    @PluginMethod
    fun executeSandbox(call: PluginCall) {
        val sandbox = requireWasmSandbox(call) ?: return
        val wasmBase64 = call.getString("wasmBase64") ?: return call.reject("wasmBase64 is required")
        val inputJson = call.getString("inputJson") ?: return call.reject("inputJson is required")

        try {
            val wasmBytes = Base64.decode(wasmBase64, Base64.DEFAULT)
            val config = toSandboxConfig(call.getObject("config"))
            val result = sandbox.`execute`(wasmBytes, inputJson, config)
            call.resolve(sandboxResultToJs(result))
        } catch (e: Exception) {
            call.reject("executeSandbox failed: ${e.message}", e)
        }
    }

    @PluginMethod
    fun executeWatSandbox(call: PluginCall) {
        val sandbox = requireWasmSandbox(call) ?: return
        val watText = call.getString("watText") ?: return call.reject("watText is required")
        val inputJson = call.getString("inputJson") ?: return call.reject("inputJson is required")

        try {
            val config = toSandboxConfig(call.getObject("config"))
            val result = sandbox.`executeWat`(watText, inputJson, config)
            call.resolve(sandboxResultToJs(result))
        } catch (e: Exception) {
            call.reject("executeWatSandbox failed: ${e.message}", e)
        }
    }

    @PluginMethod
    fun createJsRuntime(call: PluginCall) {
        try {
            jsRuntime?.close()
            jsRuntime = JsRuntime.create()
            call.resolve()
        } catch (e: Exception) {
            call.reject("createJsRuntime failed: ${e.message}", e)
        }
    }

    @PluginMethod
    fun executeJs(call: PluginCall) {
        val runtime = requireJsRuntime(call) ?: return
        val code = call.getString("code") ?: return call.reject("code is required")

        try {
            val config = toJsConfig(call.getObject("config"))
            val result = runtime.`execute`(code, config)
            call.resolve(jsResultToJs(result))
        } catch (e: Exception) {
            call.reject("executeJs failed: ${e.message}", e)
        }
    }

    @PluginMethod
    fun createPythonRuntime(call: PluginCall) {
        try {
            pythonRuntime?.close()
            pythonRuntime = PythonRuntime.create()
            call.resolve()
        } catch (e: Exception) {
            call.reject("createPythonRuntime failed: ${e.message}", e)
        }
    }

    @PluginMethod
    fun executePython(call: PluginCall) {
        val runtime = requirePythonRuntime(call) ?: return
        val code = call.getString("code") ?: return call.reject("code is required")

        try {
            val config = toPythonConfig(call.getObject("config"))
            val result = runtime.`execute`(code, config)
            call.resolve(pythonResultToJs(result))
        } catch (e: Exception) {
            call.reject("executePython failed: ${e.message}", e)
        }
    }

    private fun requireWasmSandbox(call: PluginCall): WasmSandbox? {
        val sandbox = wasmSandbox
        if (sandbox == null) {
            call.reject("WasmSandbox not initialized — call createWasmSandbox() first")
        }
        return sandbox
    }

    private fun requireJsRuntime(call: PluginCall): JsRuntime? {
        val runtime = jsRuntime
        if (runtime == null) {
            call.reject("JsRuntime not initialized - call createJsRuntime() first")
        }
        return runtime
    }

    private fun requirePythonRuntime(call: PluginCall): PythonRuntime? {
        val runtime = pythonRuntime
        if (runtime == null) {
            call.reject("PythonRuntime not initialized - call createPythonRuntime() first")
        }
        return runtime
    }

    private fun toSandboxConfig(config: JSObject?): SandboxConfig {
        val fuelLimit = config?.optLong("fuelLimit", DEFAULT_FUEL_LIMIT.toLong())?.toULong()
            ?: DEFAULT_FUEL_LIMIT
        val maxMemoryBytes =
            config?.optLong("maxMemoryBytes", DEFAULT_MAX_MEMORY_BYTES.toLong())?.toULong()
                ?: DEFAULT_MAX_MEMORY_BYTES
        val timeoutSecs = config?.optLong("timeoutSecs", DEFAULT_TIMEOUT_SECS.toLong())?.toULong()
            ?: DEFAULT_TIMEOUT_SECS
        return SandboxConfig(
            fuelLimit = fuelLimit,
            maxMemoryBytes = maxMemoryBytes,
            timeoutSecs = timeoutSecs,
        )
    }

    private fun toJsConfig(config: JSObject?): JsConfig {
        val fuelLimit =
            config?.optLong("fuelLimit", DEFAULT_JS_FUEL_LIMIT.toLong())?.toULong()
                ?: DEFAULT_JS_FUEL_LIMIT
        val maxMemoryBytes =
            config?.optLong("maxMemoryBytes", DEFAULT_JS_MAX_MEMORY_BYTES.toLong())?.toULong()
                ?: DEFAULT_JS_MAX_MEMORY_BYTES
        val timeoutSecs =
            config?.optLong("timeoutSecs", DEFAULT_JS_TIMEOUT_SECS.toLong())?.toULong()
                ?: DEFAULT_JS_TIMEOUT_SECS
        return JsConfig(
            timeoutSecs = timeoutSecs,
            fuelLimit = fuelLimit,
            maxMemoryBytes = maxMemoryBytes,
        )
    }

    private fun toPythonConfig(config: JSObject?): PythonConfig {
        val fuelLimit =
            config?.optLong("fuelLimit", DEFAULT_PYTHON_FUEL_LIMIT.toLong())?.toULong()
                ?: DEFAULT_PYTHON_FUEL_LIMIT
        val maxMemoryBytes =
            config?.optLong("maxMemoryBytes", DEFAULT_PYTHON_MAX_MEMORY_BYTES.toLong())?.toULong()
                ?: DEFAULT_PYTHON_MAX_MEMORY_BYTES
        val timeoutSecs =
            config?.optLong("timeoutSecs", DEFAULT_PYTHON_TIMEOUT_SECS.toLong())?.toULong()
                ?: DEFAULT_PYTHON_TIMEOUT_SECS
        return PythonConfig(
            timeoutSecs = timeoutSecs,
            fuelLimit = fuelLimit,
            maxMemoryBytes = maxMemoryBytes,
        )
    }

    private fun sandboxResultToJs(result: SandboxResult): JSObject {
        val payload = JSObject()
        payload.put("output", result.output)
        payload.put("fuelConsumed", result.fuelConsumed.toLong())
        payload.put("durationMs", result.durationMs.toLong())
        return payload
    }

    private fun jsResultToJs(result: JsResult): JSObject {
        val payload = JSObject()
        payload.put("output", result.output)
        if (result.error != null) {
            payload.put("error", result.error)
        }
        payload.put("fuelConsumed", result.fuelConsumed.toLong())
        payload.put("durationMs", result.durationMs.toLong())
        return payload
    }

    private fun pythonResultToJs(result: PythonResult): JSObject {
        val payload = JSObject()
        payload.put("output", result.output)
        if (result.error != null) {
            payload.put("error", result.error)
        }
        payload.put("fuelConsumed", result.fuelConsumed.toLong())
        payload.put("durationMs", result.durationMs.toLong())
        return payload
    }
}
