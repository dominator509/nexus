import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

ApprovalPrompt _prompt({
  String deviceId = '22222222-2222-4222-8222-222222222222',
  String principalId = '33333333-3333-4333-8333-333333333333',
  RiskClass risk = RiskClass.r4,
  ApprovalClass approvalClass = ApprovalClass.human,
  ApprovalPromptState state = ApprovalPromptState.pending,
  int expiresAtUnixS = 1000000000,
}) {
  return ApprovalPrompt(
    approvalId: '11111111-1111-4111-8111-111111111111',
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: principalId,
    deviceId: deviceId,
    actionId: '55555555-5555-4555-8555-555555555555',
    capabilityId: 'cap.remote.control',
    risk: risk,
    approvalClass: approvalClass,
    requester: 'alice',
    target: 'garage-door',
    externalEffects: 'opens the garage door',
    cost: 'none',
    reversible: true,
    expiresAtUnixS: expiresAtUnixS,
    state: state,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

MobileSession _session({bool revoked = false}) {
  return MobileSession(
    sessionId: '77777777-7777-4777-8777-777777777777',
    principalId: '33333333-3333-4333-8333-333333333333',
    tenantId: '44444444-4444-4444-8444-444444444444',
    deviceId: '22222222-2222-4222-8222-222222222222',
    grantFlow: GrantFlow.authorizationCode,
    strength: SessionStrength.multiFactor,
    createdAtUnixS: 0,
    expiresAtUnixS: 1000000000,
    revoked: revoked,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

DeviceBinding _binding({bool revoked = false}) {
  return DeviceBinding(
    deviceId: '22222222-2222-4222-8222-222222222222',
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: '33333333-3333-4333-8333-333333333333',
    boundAtUnixS: 0,
    revoked: revoked,
  );
}

Matcher _code(ErrorCode code) =>
    isA<Spec006Error>().having((e) => e.code, 'code', code);

void main() {
  group('ep034_failure_authority', () {
    test('wrong acting device cannot resolve the approval (AUTHORIZATION)', () {
      final service = ApprovalBindingService();
      expect(
        () => service.approve(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '99999999-9999-4999-8999-999999999999',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        ),
        throwsA(_code(ErrorCode.authorization)),
      );
    });

    test(
      'wrong acting principal cannot resolve the approval (AUTHORIZATION)',
      () {
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(),
            binding: _binding(),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '99999999-9999-4999-8999-999999999999',
            nowUnixS: 500,
          ),
          throwsA(_code(ErrorCode.authorization)),
        );
      },
    );

    test('revoked device binding is terminal for approval (AUTHORIZATION)', () {
      final service = ApprovalBindingService();
      expect(
        () => service.approve(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(revoked: true),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        ),
        throwsA(_code(ErrorCode.authorization)),
      );
    });

    test('revoked session cannot authorize a consequential action', () {
      final service = ApprovalBindingService();
      expect(
        () => service.approve(
          prompt: _prompt(),
          session: _session(revoked: true),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        ),
        throwsA(_code(ErrorCode.authorization)),
      );
    });

    test('R4 approval with POLICY class never mints authority (POLICY)', () {
      final service = ApprovalBindingService();
      expect(
        () => service.approve(
          prompt: _prompt(approvalClass: ApprovalClass.policy),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        ),
        throwsA(_code(ErrorCode.policy)),
      );
    });

    test('expired approval is never actionable (POLICY)', () {
      final service = ApprovalBindingService();
      expect(
        () => service.approve(
          prompt: _prompt(expiresAtUnixS: 100),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        ),
        throwsA(_code(ErrorCode.policy)),
      );
    });

    test('offline R3 control is denied from cached policy (POLICY)', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.lock.open',
          risk: RiskClass.r3,
          policyVersion: 'policy-v1',
          cachedAtUnixS: 0,
          expiresAtUnixS: 1000000000,
        ),
      );
      final cache = OfflinePolicyCache(store: store);
      expect(
        () => cache.decide(
          capabilityId: 'cap.lock.open',
          risk: RiskClass.r3,
          nowUnixS: 500,
        ),
        throwsA(_code(ErrorCode.policy)),
      );
    });

    test('stale cached allowance is never actionable (POLICY)', () {
      final store = InMemoryOfflinePolicyStore();
      store.put(
        CachedPolicyEntry(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          policyVersion: 'policy-v0',
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
        throwsA(_code(ErrorCode.policy)),
      );
    });

    test('unknown capability fails closed offline (POLICY)', () {
      final cache = OfflinePolicyCache();
      expect(
        () => cache.decide(
          capabilityId: 'cap.fabricated.thing',
          risk: RiskClass.r1,
          nowUnixS: 500,
        ),
        throwsA(_code(ErrorCode.policy)),
      );
    });
  });
}
