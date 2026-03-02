require 'json'

package = JSON.parse(File.read(File.join(File.dirname(__FILE__), 'package.json')))

Pod::Spec.new do |s|
  s.name = 'CapacitorWasmAgentTools'
  s.version = package['version']
  s.summary = package['description']
  s.license = package['license']
  s.homepage = 'https://github.com/techxagon/capacitor-wasm-agent-tools'
  s.author = 'Techxagon'
  s.source = { :git => 'https://github.com/techxagon/capacitor-wasm-agent-tools.git', :tag => s.version.to_s }

  s.source_files = 'ios/Sources/**/*.swift'
  s.ios.deployment_target = '14.0'

  s.dependency 'Capacitor'
  s.swift_version = '5.9'
  s.vendored_frameworks = 'ios/Frameworks/WasmAgentToolsFFI.xcframework'

  s.pod_target_xcconfig = {
    'OTHER_SWIFT_FLAGS[sdk=iphoneos*]' => '$(inherited) -Xcc -fmodule-map-file=${PODS_TARGET_SRCROOT}/ios/Frameworks/WasmAgentToolsFFI.xcframework/ios-arm64/Headers/wasm_agent_tools_ffiFFI.modulemap',
    'OTHER_SWIFT_FLAGS[sdk=iphonesimulator*]' => '$(inherited) -Xcc -fmodule-map-file=${PODS_TARGET_SRCROOT}/ios/Frameworks/WasmAgentToolsFFI.xcframework/ios-arm64-simulator/Headers/wasm_agent_tools_ffiFFI.modulemap',
  }
end
