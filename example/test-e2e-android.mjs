#!/usr/bin/env node

import { execSync } from 'child_process'
import fs from 'fs'
import http from 'http'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const ADB = process.env.ADB_PATH || 'adb'
const BUNDLE_ID = 'io.t6x.wasmagenttools.test'
const RUNNER_PORT = 8099
const TOTAL_TESTS = 25
const TIMEOUT_MS = 240_000

const TEST_NAMES = {
  sandbox_create: 'WASM Sandbox: create succeeds',
  sandbox_echo: 'WASM Sandbox: executeWat echo round-trips JSON',
  sandbox_fuel: 'WASM Sandbox: fuel limit stops infinite loop',
  sandbox_timeout: 'WASM Sandbox: timeout stops infinite loop',
  sandbox_fuel_tracking: 'WASM Sandbox: fuel tracking reports consumption',
  sandbox_invalid_wasm: 'WASM Sandbox: invalid WASM bytes rejected',
  sandbox_missing_exports: 'WASM Sandbox: missing exports rejected',
  sandbox_wat_convenience: 'WASM Sandbox: WAT convenience with different payload',
  sandbox_memory_limit: 'WASM Sandbox: memory limit enforced',
  js_create: 'JS Runtime: create succeeds',
  js_expression: 'JS Runtime: simple expression returns result',
  js_console_log: 'JS Runtime: console.log captures stdout',
  js_console_error: 'JS Runtime: console.error does not crash',
  js_multi_statement: 'JS Runtime: multi-statement execution',
  js_timeout: 'JS Runtime: timeout stops infinite loop',
  js_throw: 'JS Runtime: throw populates error field',
  js_no_require: 'JS Runtime: require is not available',
  python_create: 'Python Runtime: create succeeds',
  python_print: 'Python Runtime: print returns stdout',
  python_multi_print: 'Python Runtime: multiple prints concatenated',
  python_math: 'Python Runtime: stdlib math import works',
  python_json: 'Python Runtime: stdlib json import works',
  python_timeout: 'Python Runtime: timeout stops infinite loop',
  python_blocked_import: 'Python Runtime: blocked imports fail',
  python_raise: 'Python Runtime: raised exceptions surface in error',
}

function adb(args, opts = {}) {
  const serial = process.env.ANDROID_SERIAL ? `-s ${process.env.ANDROID_SERIAL}` : ''
  return execSync(`${ADB} ${serial} ${args}`, { encoding: 'utf8', timeout: 60_000, ...opts }).trim()
}

function connectedDevice() {
  const output = execSync(`${ADB} devices`, { encoding: 'utf8' })
  const lines = output.split('\n').slice(1).filter(line => line.includes('\tdevice'))
  if (lines.length === 0) {
    return null
  }
  return lines[0].split('\t')[0].trim()
}

function startResultServer() {
  const received = new Map()

  return new Promise((resolve, reject) => {
    const server = http.createServer((req, res) => {
      res.setHeader('Access-Control-Allow-Origin', '*')
      res.setHeader('Access-Control-Allow-Methods', 'POST, OPTIONS')
      res.setHeader('Access-Control-Allow-Headers', 'Content-Type')

      if (req.method === 'OPTIONS') {
        res.writeHead(204)
        res.end()
        return
      }

      let body = ''
      req.on('data', chunk => { body += chunk })
      req.on('end', () => {
        try {
          const payload = JSON.parse(body || '{}')
          if (req.url === '/__wasmtools_result') {
            received.set(payload.id, payload)
            res.writeHead(200)
            res.end('ok')
            return
          }
          if (req.url === '/__wasmtools_done') {
            res.writeHead(200)
            res.end('ok')
            server.close()
            resolve({ received, summary: payload })
            return
          }
        } catch {
        }

        res.writeHead(400)
        res.end()
      })
    })

    server.listen(RUNNER_PORT, '0.0.0.0')
    server.on('error', reject)
    setTimeout(() => {
      server.close()
      reject(new Error(`Timed out after ${TIMEOUT_MS / 1000}s with ${received.size}/${TOTAL_TESTS} results`))
    }, TIMEOUT_MS)
  })
}

function maybeBuildAndInstall() {
  const androidDir = path.join(__dirname, 'android')
  if (!fs.existsSync(androidDir)) {
    console.log('  → No example/android project found. Reusing any installed app.')
    return
  }

  const apkPath = path.join(androidDir, 'app/build/outputs/apk/debug/app-debug.apk')
  console.log('  → Building example APK...')
  execSync('./gradlew assembleDebug', {
    cwd: androidDir,
    encoding: 'utf8',
    timeout: 300_000,
    stdio: ['ignore', 'pipe', 'pipe'],
  })

  if (!fs.existsSync(apkPath)) {
    throw new Error('Expected app-debug.apk after assembleDebug')
  }

  try {
    adb(`uninstall ${BUNDLE_ID}`)
  } catch {
  }
  adb(`install -r "${apkPath}"`)
  console.log('  → Installed fresh APK')
}

async function main() {
  console.log('\nAndroid E2E: capacitor-wasm-agent-tools\n')

  const device = connectedDevice()
  if (!device) {
    throw new Error('No Android device detected')
  }
  process.env.ANDROID_SERIAL = device
  console.log(`  → Using device ${device}`)

  adb(`reverse tcp:${RUNNER_PORT} tcp:${RUNNER_PORT}`)
  maybeBuildAndInstall()

  console.log(`  → Waiting for HTTP results on :${RUNNER_PORT}`)
  const resultPromise = startResultServer()

  adb(`shell am start -n "${BUNDLE_ID}/.MainActivity"`)
  const { received, summary } = await resultPromise

  let passed = 0
  for (const id of Object.keys(TEST_NAMES)) {
    const result = received.get(id)
    if (result && result.status === 'pass') {
      passed += 1
      console.log(`  PASS ${TEST_NAMES[id]}`)
    } else {
      console.log(`  FAIL ${TEST_NAMES[id]}${result && result.error ? ` — ${result.error}` : ' — no result received'}`)
    }
  }

  console.log(`\n  Result: ${passed}/${TOTAL_TESTS} passed`)
  if (summary) {
    console.log(`  App reported: ${summary.passed}/${summary.total} passed`)
  }

  process.exit(passed === TOTAL_TESTS ? 0 : 1)
}

main().catch(error => {
  console.error(`\nFatal: ${error.message}`)
  process.exit(1)
})
