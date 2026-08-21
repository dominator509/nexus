/// EP-034 M1 SecureStore contract (SPEC-017).
///
/// Offline clients cache only explicitly allowed data and encrypt it
/// with platform keys (SPEC-017 behavior 6). The store exposes typed
/// get/set/delete with an allowlist; tokens and secrets are refused
/// by the same boundary used by the web/desktop preference layer.
library;

import 'errors.dart';
import 'validate.dart';

/// Canonical secure store entry kinds.
enum SecureStoreEntryKind {
  session('SESSION'),
  cachedPolicy('CACHED_POLICY'),
  cachedContext('CACHED_CONTEXT'),
  passkeyHandle('PASSKEY_HANDLE'),
  pushEndpoint('PUSH_ENDPOINT');

  const SecureStoreEntryKind(this.wire);
  final String wire;
}

/// A SecureStore entry: typed, keyed, device-bound. The value is
/// opaque to the contract layer; the native secure enclave/keystore
/// owns encryption.
class SecureStoreEntry {
  const SecureStoreEntry({
    required this.key,
    required this.kind,
    required this.deviceId,
    required this.tenantId,
    required this.updatedAtUnixS,
  });

  factory SecureStoreEntry.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'key',
      'kind',
      'device_id',
      'tenant_id',
      'updated_at_unix_s',
    };
    rejectUnknownKeys(json, allowed);
    final updated = json['updated_at_unix_s'];
    return SecureStoreEntry(
      key: requireString(json, 'key'),
      kind: requireEnum(
        json,
        'kind',
        SecureStoreEntryKind.values,
        wireOf: (v) => v.wire,
      ),
      deviceId: requireUuid(json, 'device_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      updatedAtUnixS: updated is int
          ? updated
          : throw Spec006Error(
              ErrorCode.validation,
              'updated_at_unix_s must be an integer',
            ),
    );
  }

  final String key;
  final SecureStoreEntryKind kind;
  final String deviceId;
  final String tenantId;
  final int updatedAtUnixS;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'key': key,
    'kind': kind.wire,
    'device_id': deviceId,
    'tenant_id': tenantId,
    'updated_at_unix_s': updatedAtUnixS,
  };
}

/// SecureStore: the mobile secure local storage boundary (SPEC-017
/// behavior 6). Only explicitly allowed entry kinds may be cached;
/// arbitrary blobs are refused.
class SecureStore {
  const SecureStore({
    required this.deviceId,
    required this.tenantId,
    required this.entries,
  });

  factory SecureStore.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{'device_id', 'tenant_id', 'entries'};
    rejectUnknownKeys(json, allowed);
    final rawEntries = json['entries'];
    if (rawEntries is! List) {
      throw Spec006Error(ErrorCode.validation, 'entries must be a list');
    }
    final entries = rawEntries
        .map(
          (e) => e is Map<String, dynamic>
              ? SecureStoreEntry.fromJson(e)
              : throw Spec006Error(
                  ErrorCode.validation,
                  'entry must be an object',
                ),
        )
        .toList(growable: false);
    return SecureStore(
      deviceId: requireUuid(json, 'device_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      entries: entries,
    );
  }

  final String deviceId;
  final String tenantId;
  final List<SecureStoreEntry> entries;

  /// The allowlist of entry kinds that may be cached offline.
  static const Set<SecureStoreEntryKind> allowedOfflineKinds = {
    SecureStoreEntryKind.cachedPolicy,
    SecureStoreEntryKind.cachedContext,
    SecureStoreEntryKind.session,
    SecureStoreEntryKind.pushEndpoint,
  };

  Map<String, dynamic> toJson() => <String, dynamic>{
    'device_id': deviceId,
    'tenant_id': tenantId,
    'entries': entries.map((e) => e.toJson()).toList(growable: false),
  };
}
