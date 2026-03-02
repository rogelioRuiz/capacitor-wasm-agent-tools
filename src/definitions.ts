export interface SandboxConfig {
  fuelLimit?: number
  maxMemoryBytes?: number
  timeoutSecs?: number
}

export interface SandboxResult {
  output: string
  fuelConsumed: number
  durationMs: number
}

export interface JsConfig {
  fuelLimit?: number
  maxMemoryBytes?: number
  timeoutSecs?: number
}

export interface JsResult {
  output: string
  error?: string
  fuelConsumed: number
  durationMs: number
}

export interface PythonConfig {
  fuelLimit?: number
  maxMemoryBytes?: number
  timeoutSecs?: number
}

export interface PythonResult {
  output: string
  error?: string
  fuelConsumed: number
  durationMs: number
}

export interface WasmAgentToolsPlugin {
  createWasmSandbox(): Promise<void>
  executeSandbox(options: {
    wasmBase64: string
    inputJson: string
    config?: SandboxConfig
  }): Promise<SandboxResult>
  executeWatSandbox(options: {
    watText: string
    inputJson: string
    config?: SandboxConfig
  }): Promise<SandboxResult>

  createJsRuntime(): Promise<void>
  executeJs(options: {
    code: string
    config?: JsConfig
  }): Promise<JsResult>

  createPythonRuntime(): Promise<void>
  executePython(options: {
    code: string
    config?: PythonConfig
  }): Promise<PythonResult>
}
