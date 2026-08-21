/// EP-034 M2 offline cached-policy behavior (SPEC-017 behavior 6;
/// node contract: offline low-risk controls follow cached policy).
///
/// Only explicitly allowed data is cached. Every cached allowance
/// carries its policy version and a freshness window; a stale entry
/// is never actionable. High-risk (R3/R4) controls never run from
/// cached policy - they require online authorization. Unknown
/// capabilities and stale entries fail closed with typed SPEC-006
/// errors.
library;

import 'package:nexus_mobile/nexus_mobile.dart';

/// A cached allowance for one low-risk control. Only explicitly
/// allowed data is cached (SPEC-017 behavior 6).
class CachedPolicyEntry {
  const CachedPolicyEntry({
    required this.capabilityId,
    required this.risk,
    required this.policyVersion,
    required this.cachedAtUnixS,
    required this.expiresAtUnixS,
  });

  factory CachedPolicyEntry.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'capability_id',
      'risk',
      'policy_version',
      'cached_at_unix_s',
      'expires_at_unix_s',
    };
    rejectUnknownKeys(json, allowed);
    final cachedAt = json['cached_at_unix_s'];
    final expiresAt = json['expires_at_unix_s'];
    return CachedPolicyEntry(
      capabilityId: requireString(json, 'capability_id'),
      risk: requireEnum(json, 'risk', RiskClass.values, wireOf: (v) => v.wire),
      policyVersion: requireString(json, 'policy_version'),
      cachedAtUnixS: cachedAt is int
          ? cachedAt
          : throw Spec006Error(
              ErrorCode.validation,
              'cached_at_unix_s must be an integer',
            ),
      expiresAtUnixS: expiresAt is int
          ? expiresAt
          : throw Spec006Error(
              ErrorCode.validation,
              'expires_at_unix_s must be an integer',
            ),
    );
  }

  final String capabilityId;
  final RiskClass risk;
  final String policyVersion;
  final int cachedAtUnixS;
  final int expiresAtUnixS;

  /// Fresh while now <= expiry; at the exact expiry instant the entry
  /// is still actionable, after it is stale.
  bool isFreshAt(int nowUnixS) => nowUnixS <= expiresAtUnixS;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'capability_id': capabilityId,
    'risk': risk.wire,
    'policy_version': policyVersion,
    'cached_at_unix_s': cachedAtUnixS,
    'expires_at_unix_s': expiresAtUnixS,
  };
}

/// Port for offline cache storage. The M2 in-memory implementation is
/// the real M2 layer for deterministic invariants; platform secure
/// storage is a later milestone.
abstract class OfflinePolicyStore {
  CachedPolicyEntry? find(String capabilityId);
  void put(CachedPolicyEntry entry);
  void clear();
}

/// In-memory offline policy store (M2 implementation).
class InMemoryOfflinePolicyStore implements OfflinePolicyStore {
  final Map<String, CachedPolicyEntry> _entries = <String, CachedPolicyEntry>{};

  @override
  CachedPolicyEntry? find(String capabilityId) => _entries[capabilityId];

  @override
  void put(CachedPolicyEntry entry) {
    _entries[entry.capabilityId] = entry;
  }

  @override
  void clear() => _entries.clear();
}

/// Allowance outcome: a low-risk control may run offline under the
/// cached policy version at the given instant.
class OfflineAllowance {
  const OfflineAllowance({
    required this.capabilityId,
    required this.policyVersion,
    required this.allowedAtUnixS,
  });

  final String capabilityId;
  final String policyVersion;
  final int allowedAtUnixS;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'capability_id': capabilityId,
    'policy_version': policyVersion,
    'allowed_at_unix_s': allowedAtUnixS,
  };
}

/// Decides offline control execution from cached policy. Denials are
/// typed SPEC-006 errors; no hidden fallback.
class OfflinePolicyCache {
  OfflinePolicyCache({OfflinePolicyStore? store})
    : _store = store ?? InMemoryOfflinePolicyStore();

  final OfflinePolicyStore _store;

  OfflineAllowance decide({
    required String capabilityId,
    required RiskClass risk,
    required int nowUnixS,
  }) {
    if (risk == RiskClass.r3 || risk == RiskClass.r4) {
      throw Spec006Error(
        ErrorCode.policy,
        'high-risk control requires online authorization',
      );
    }
    final entry = _store.find(capabilityId);
    if (entry == null) {
      throw Spec006Error(
        ErrorCode.policy,
        'no cached allowance for capability',
      );
    }
    if (!entry.isFreshAt(nowUnixS)) {
      throw Spec006Error(ErrorCode.policy, 'cached allowance is stale');
    }
    if (entry.risk != risk) {
      throw Spec006Error(
        ErrorCode.policy,
        'cached allowance risk does not match request',
      );
    }
    return OfflineAllowance(
      capabilityId: capabilityId,
      policyVersion: entry.policyVersion,
      allowedAtUnixS: nowUnixS,
    );
  }
}
