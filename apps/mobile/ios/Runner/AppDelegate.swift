import Flutter
import UIKit
import LocalAuthentication

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    bindSecurityChannel(engineBridge)
  }

  /// AUD-040: bind the REAL native security surface behind the
  /// `nexus.mobile/security` MethodChannel. Every operation FAILS
  /// CLOSED: no biometric hardware -> honest false; a cancelled/failed
  /// LocalAuthentication prompt -> denial; a Keychain error -> nil /
  /// false. The Dart shell never fabricates a platform success.
  private func bindSecurityChannel(_ engineBridge: FlutterImplicitEngineBridge) {
    let channel = FlutterMethodChannel(
      name: "nexus.mobile/security",
      binaryMessenger: engineBridge.binaryMessenger
    )
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self = self else {
        result(FlutterMethodNotImplemented)
        return
      }
      switch call.method {
      case "biometricCapability":
        result(["available": self.biometricAvailable()])
      case "biometricVerify":
        let reason = (call.arguments as? [String: Any])?["reason"] as? String ?? ""
        self.verifyBiometric(reason: reason) { verified, correlation in
          result(["verified": verified, "correlation": correlation])
        }
      case "secureStore":
        guard
          let args = call.arguments as? [String: Any],
          let key = args["key"] as? String,
          let value = args["value"] as? String
        else {
          result(["stored": false])
          return
        }
        result(["stored": self.storeSecret(key: key, value: value)])
      case "secureRead":
        guard
          let args = call.arguments as? [String: Any],
          let key = args["key"] as? String
        else {
          result(["value": nil])
          return
        }
        result(["value": self.readSecret(key: key)])
      case "secureDelete":
        guard
          let args = call.arguments as? [String: Any],
          let key = args["key"] as? String
        else {
          result(["deleted": false])
          return
        }
        result(["deleted": self.deleteSecret(key: key)])
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  private func biometricAvailable() -> Bool {
    let context = LAContext()
    var error: NSError?
    return context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error)
  }

  private func verifyBiometric(
    reason: String,
    completion: @escaping (Bool, String) -> Void
  ) {
    let context = LAContext()
    var error: NSError?
    guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) else {
      completion(false, "")
      return
    }
    let correlation = UUID().uuidString.lowercased()
    context.evaluatePolicy(
      .deviceOwnerAuthenticationWithBiometrics,
      localizedReason: reason.isEmpty ? "Verify to approve this action" : reason
    ) { success, _ in
      DispatchQueue.main.async {
        completion(success, success ? correlation : "")
      }
    }
  }

  private func storeSecret(key: String, value: String) -> Bool {
    let data = Data(value.utf8)
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: "nexus.secure.\(key)",
      kSecValueData as String: data,
      kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
    ]
    SecItemDelete(query as CFDictionary)
    let status = SecItemAdd(query as CFDictionary, nil)
    return status == errSecSuccess
  }

  private func readSecret(key: String) -> String? {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: "nexus.secure.\(key)",
      kSecReturnData as String: true,
      kSecMatchLimit as String: kSecMatchLimitOne,
    ]
    var item: CFTypeRef?
    let status = SecItemCopyMatching(query as CFDictionary, &item)
    guard status == errSecSuccess, let data = item as? Data else {
      return nil
    }
    return String(data: data, encoding: .utf8)
  }

  private func deleteSecret(key: String) -> Bool {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: "nexus.secure.\(key)",
    ]
    let status = SecItemDelete(query as CFDictionary)
    return status == errSecSuccess || status == errSecItemNotFound
  }
}
