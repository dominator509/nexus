/// EP-034 native security channel contract (AUD-040).
///
/// The Flutter app previously advertised passkeys, biometrics, and
/// secure local storage in its pubspec description while the Android
/// surface was a stock four-line FlutterActivity and no native
/// security implementation existed. This contract defines the REAL
/// MethodChannel surface the Dart shell uses to reach platform
/// security primitives, and it FAILS CLOSED at every boundary:
///
/// - a missing/unbound channel is Unavailable, never a silent success;
/// - a platform error response is Unavailable (the failure propagates,
///   never masked as a negative result);
/// - a malformed response (missing/typed-wrong fields) is Unavailable;
/// - a null/absent biometric result is never treated as a success;
/// - hardware biometric verification is NEVER asserted from Dart - the
///   platform reports its own capability truthfully and the shell
///   treats "no biometric hardware" as an honest Unavailable.
///
/// The Android side implements the documented platform surface
/// (androidx.biometric BiometricManager/BiometricPrompt) behind this
/// channel; the iOS side mirrors it behind LocalAuthentication. This
/// file is the single source of the channel name and method contract.
library;

import 'dart:async';
import 'package:flutter/services.dart';

/// Canonical MethodChannel name (AUD-040). The native side MUST bind
/// exactly this name; a channel that is not bound is Unavailable.
const String kNexusSecurityChannel = 'nexus.mobile/security';

/// Canonical method names on [kNexusSecurityChannel].
class NexusSecurityMethods {
  NexusSecurityMethods._();

  /// Returns `{"available": bool}` - whether the platform has a
  /// biometric authenticator (Android BiometricManager / iOS
  /// LocalAuthentication). Never asserted from Dart.
  static const String biometricCapability = 'biometricCapability';

  /// Returns `{"verified": bool, "correlation": "<uuid>"}` after a
  /// REAL platform biometric prompt (crypto-based). A false or absent
  /// result is NEVER a success.
  static const String biometricVerify = 'biometricVerify';

  /// Stores a secret in the platform secure store (Android Keystore /
  /// iOS Keychain). Returns `{"stored": true}` only after the native
  /// layer confirms the write.
  static const String secureStore = 'secureStore';

  /// Reads a secret from the platform secure store. Absent entries
  /// are an honest null result (`{"value": null}`), never an error.
  static const String secureRead = 'secureRead';

  /// Removes a secret from the platform secure store.
  static const String secureDelete = 'secureDelete';
}

/// Truthful result of a native capability probe.
class BiometricCapability {
  const BiometricCapability({required this.available});
  final bool available;

  /// FAILS CLOSED: any non-boolean/absent `available` is treated as
  /// no biometric capability (never an invented success).
  factory BiometricCapability.fromPlatform(Object? raw) {
    if (raw is Map) {
      final value = raw['available'];
      if (value is bool) {
        return BiometricCapability(available: value);
      }
    }
    return const BiometricCapability(available: false);
  }
}

/// Truthful result of a native biometric verification.
class BiometricVerification {
  const BiometricVerification({
    required this.verified,
    required this.correlation,
  });
  final bool verified;
  final String correlation;

  /// FAILS CLOSED: verification is true ONLY when the platform
  /// returned `verified == true` AND a non-empty correlation. Any
  /// other shape (error, absent, null, malformed) is a denial.
  factory BiometricVerification.fromPlatform(Object? raw) {
    if (raw is Map) {
      final verified = raw['verified'];
      final correlation = raw['correlation'];
      if (verified is bool &&
          verified == true &&
          correlation is String &&
          correlation.isNotEmpty) {
        return BiometricVerification(verified: true, correlation: correlation);
      }
    }
    return const BiometricVerification(verified: false, correlation: '');
  }
}

/// Secure-store read result. A null value is an honest "absent".
class SecureReadResult {
  const SecureReadResult({required this.value});
  final String? value;
}

/// Native security surface port. The default implementation talks to
/// the REAL platform channel; tests inject a bounded fake that
/// answers with controlled platform-shaped payloads (never mocks of
/// the production channel - the failure suite drives the real
/// MethodChannel through TestDefaultBinaryMessenger).
abstract class NativeSecuritySurface {
  Future<BiometricCapability> biometricCapability();
  Future<BiometricVerification> biometricVerify({required String reason});
  Future<bool> secureStore({required String key, required String value});
  Future<SecureReadResult> secureRead({required String key});
  Future<bool> secureDelete({required String key});
}

/// Production surface over the real [MethodChannel]. Every operation
/// FAILS CLOSED on MissingPluginException, PlatformException, or a
/// malformed response.
class MethodChannelNativeSecuritySurface implements NativeSecuritySurface {
  MethodChannelNativeSecuritySurface({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel(kNexusSecurityChannel);

  final MethodChannel _channel;

  @override
  Future<BiometricCapability> biometricCapability() async {
    try {
      final raw = await _channel.invokeMethod<Object?>(
        NexusSecurityMethods.biometricCapability,
      );
      return BiometricCapability.fromPlatform(raw);
    } on MissingPluginException {
      // Channel not bound on this platform: honest Unavailable, never
      // a fabricated capability.
      return const BiometricCapability(available: false);
    } on PlatformException {
      return const BiometricCapability(available: false);
    }
  }

  @override
  Future<BiometricVerification> biometricVerify({
    required String reason,
  }) async {
    try {
      final raw = await _channel.invokeMethod<Object?>(
        NexusSecurityMethods.biometricVerify,
        <String, Object?>{'reason': reason},
      );
      return BiometricVerification.fromPlatform(raw);
    } on MissingPluginException {
      return const BiometricVerification(verified: false, correlation: '');
    } on PlatformException {
      return const BiometricVerification(verified: false, correlation: '');
    }
  }

  @override
  Future<bool> secureStore({required String key, required String value}) async {
    try {
      final raw = await _channel.invokeMethod<Object?>(
        NexusSecurityMethods.secureStore,
        <String, Object?>{'key': key, 'value': value},
      );
      // FAILS CLOSED: only an explicit `stored == true` is success.
      return raw is Map && raw['stored'] == true;
    } on MissingPluginException {
      return false;
    } on PlatformException {
      return false;
    }
  }

  @override
  Future<SecureReadResult> secureRead({required String key}) async {
    try {
      final raw = await _channel.invokeMethod<Object?>(
        NexusSecurityMethods.secureRead,
        <String, Object?>{'key': key},
      );
      if (raw is Map) {
        final value = raw['value'];
        if (value == null || value is String) {
          return SecureReadResult(value: value as String?);
        }
      }
      // Malformed: fail closed as absent, never an invented value.
      return const SecureReadResult(value: null);
    } on MissingPluginException {
      return const SecureReadResult(value: null);
    } on PlatformException {
      return const SecureReadResult(value: null);
    }
  }

  @override
  Future<bool> secureDelete({required String key}) async {
    try {
      final raw = await _channel.invokeMethod<Object?>(
        NexusSecurityMethods.secureDelete,
        <String, Object?>{'key': key},
      );
      return raw is Map && raw['deleted'] == true;
    } on MissingPluginException {
      return false;
    } on PlatformException {
      return false;
    }
  }
}
