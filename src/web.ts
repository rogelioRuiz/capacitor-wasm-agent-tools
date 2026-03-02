import { registerPlugin, WebPlugin } from '@capacitor/core'

import type { JsResult, PythonResult, SandboxResult, WasmAgentToolsPlugin } from './definitions'

export class WasmAgentToolsWeb extends WebPlugin implements WasmAgentToolsPlugin {
  async createWasmSandbox(): Promise<void> {
    throw this.unavailable('WasmAgentTools is only available on native platforms.')
  }

  async executeSandbox(): Promise<SandboxResult> {
    throw this.unavailable('WasmAgentTools is only available on native platforms.')
  }

  async executeWatSandbox(): Promise<SandboxResult> {
    throw this.unavailable('WasmAgentTools is only available on native platforms.')
  }

  async createJsRuntime(): Promise<void> {
    throw this.unavailable('WasmAgentTools is only available on native platforms.')
  }

  async executeJs(): Promise<JsResult> {
    throw this.unavailable('WasmAgentTools is only available on native platforms.')
  }

  async createPythonRuntime(): Promise<void> {
    throw this.unavailable('WasmAgentTools is only available on native platforms.')
  }

  async executePython(): Promise<PythonResult> {
    throw this.unavailable('WasmAgentTools is only available on native platforms.')
  }
}

export const WasmAgentTools = registerPlugin<WasmAgentToolsPlugin>('WasmAgentTools', {
  web: () => Promise.resolve(new WasmAgentToolsWeb()),
})
