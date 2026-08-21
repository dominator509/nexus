import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

void main() {
  group('ep034_unit_offline_policy', () {
    test('offline low-risk control with fresh cached allowance is allowed', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 1000,
        ),
      );
      final cache = OfflinePolicyCache(store: store);
      final allowance = cache.decide(
        capabilityId: 'cap.light.toggle',
        risk: RiskClass.r1,
        nowUnixS: 500,
      );
      expect(allowance.capabilityId, 'cap.light.toggle');
      expect(allowance.policyVersion, 'policy-v1');
      expect(allowance.allowedAtUnixS, 500);
    });

    test('unknown capability is denied (POLICY)', () {
      final cache = OfflinePolicyCache();
      expect(
        () => cache.decide(
          capabilityId: 'cap.unknown.thing',
          risk: RiskClass.r1,
          nowUnixS: 500,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test('stale cached allowance is denied (POLICY)', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 100,
        ),
      );
      final cache = OfflinePolicyCache(store: store);
      expect(
        () => cache.decide(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          nowUnixS: 500,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test('boundary: allowance fresh exactly at expiry instant', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 500,
        ),
      );
      final cache = OfflinePolicyCache(store: store);
      final allowance = cache.decide(
        capabilityId: 'cap.light.toggle',
        risk: RiskClass.r1,
        nowUnixS: 500,
      );
      expect(allowance.allowedAtUnixS, 500);
    });

    test('R3 control never runs from cached policy (POLICY)', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.lock.open',
          risk: RiskClass.r3,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 1000,
        ),
      );
      final cache = OfflinePolicyCache(store: store);
      expect(
        () => cache.decide(
          capabilityId: 'cap.lock.open',
          risk: RiskClass.r3,
          nowUnixS: 500,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test('R4 control never runs from cached policy (POLICY)', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.money.send',
          risk: RiskClass.r4,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 1000,
        ),
      );
      final cache = OfflinePolicyCache(store: store);
      expect(
        () => cache.decide(
          capabilityId: 'cap.money.send',
          risk: RiskClass.r4,
          nowUnixS: 500,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test('cache entry cannot upgrade requested risk class', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 1000,
        ),
      );
      final cache = OfflinePolicyCache(store: store);
      expect(
        () => cache.decide(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r2,
          nowUnixS: 500,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test('CachedPolicyEntry round-trips wire JSON', () {
      const entry = CachedPolicyEntry(
        capabilityId: 'cap.light.toggle',
        risk: RiskClass.r1,
        policyVersion: 'policy-v1',
        cachedAtUnixS: 0,
        expiresAtUnixS: 1000,
      );
      final parsed = CachedPolicyEntry.fromJson(entry.toJson());
      expect(parsed.capabilityId, entry.capabilityId);
      expect(parsed.risk, entry.risk);
      expect(parsed.policyVersion, entry.policyVersion);
      expect(parsed.cachedAtUnixS, entry.cachedAtUnixS);
      expect(parsed.expiresAtUnixS, entry.expiresAtUnixS);
    });

    test('CachedPolicyEntry rejects unknown fields (VOCABULARY)', () {
      expect(
        () => CachedPolicyEntry.fromJson(<String, dynamic>{
          'capability_id': 'cap.light.toggle',
          'risk': 'R1',
          'policy_version': 'policy-v1',
          'cached_at_unix_s': 0,
          'expires_at_unix_s': 1000,
          'fabricated_field': 'x',
        }),
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.vocabulary,
          ),
        ),
      );
    });

    test('CachedPolicyEntry rejects unknown risk enum (VOCABULARY)', () {
      expect(
        () => CachedPolicyEntry.fromJson(<String, dynamic>{
          'capability_id': 'cap.light.toggle',
          'risk': 'R9',
          'policy_version': 'policy-v1',
          'cached_at_unix_s': 0,
          'expires_at_unix_s': 1000,
        }),
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.vocabulary,
          ),
        ),
      );
    });

    test('in-memory store is the M2 implementation of the cache port', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        const CachedPolicyEntry(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 1000,
        ),
      );
      expect(store.find('cap.light.toggle'), isNotNull);
      expect(store.find('cap.other'), isNull);
      store.clear();
      expect(store.find('cap.light.toggle'), isNull);
    });
  });
}
