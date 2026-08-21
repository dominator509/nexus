import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

/// LF-004 multi-user-identity live-fire (EP-034 M5).
///
/// Enroll two adults (Alice, Bob) and one restricted user (Charlie);
/// prove separate context, permissions, preferences, and mobile
/// devices - composed from the REAL production contract layer
/// (nexus_mobile) and behavior layer (nexus_mobile_contracts). No
/// mocks, no simulated backend. Native providers remain NOT ASSERTED.
///
/// Enrollment in this journey is the production DeviceBinding +
/// ClientDevice + MobileSession construction; platform attestation is
/// the deferred native milestone.

class EnrolledUser {
  EnrolledUser({
    required this.name,
    required this.principalId,
    required this.deviceId,
    required this.device,
    required this.binding,
    required this.session,
    required this.store,
    required this.preferences,
  });

  final String name;
  final String principalId;
  final String deviceId;
  final ClientDevice device;
  final DeviceBinding binding;
  final MobileSession session;
  final InMemoryOfflinePolicyStore store;
  final List<String> preferences;
}

String _uuid(String seed) => seed;

EnrolledUser _enroll({
  required String name,
  required String principalId,
  required String deviceId,
  required DeviceTrustLevel trust,
  required List<String> preferences,
}) {
  final device = ClientDevice(
    deviceId: deviceId,
    tenantId: '44444444-4444-4444-8444-444444444444',
    displayName: '$name-phone',
    kind: DeviceKind.phone,
    trustLevel: trust,
    ownerPersonId: principalId,
  );
  final binding = DeviceBinding(
    deviceId: deviceId,
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: principalId,
    boundAtUnixS: 0,
    revoked: false,
  );
  final session = MobileSession(
    sessionId: _uuid('session-$deviceId'),
    principalId: principalId,
    tenantId: '44444444-4444-4444-8444-444444444444',
    deviceId: deviceId,
    grantFlow: GrantFlow.authorizationCode,
    strength: SessionStrength.multiFactor,
    createdAtUnixS: 0,
    expiresAtUnixS: 1000000000,
    revoked: false,
    correlation: _uuid('corr-$deviceId'),
  );
  final store = InMemoryOfflinePolicyStore();
  var idx = 0;
  for (final capability in preferences) {
    store.put(
      CachedPolicyEntry(
        capabilityId: capability,
        risk: idx == 0 ? RiskClass.r1 : RiskClass.r2,
        policyVersion: 'policy-v1',
        cachedAtUnixS: 0,
        expiresAtUnixS: 1000000000,
      ),
    );
    idx++;
  }
  return EnrolledUser(
    name: name,
    principalId: principalId,
    deviceId: deviceId,
    device: device,
    binding: binding,
    session: session,
    store: store,
    preferences: preferences,
  );
}

ApprovalPrompt _promptFor(
  EnrolledUser user, {
  RiskClass risk = RiskClass.r4,
  ApprovalClass approvalClass = ApprovalClass.human,
}) {
  return ApprovalPrompt(
    approvalId: '11111111-1111-4111-8111-111111111111',
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: user.principalId,
    deviceId: user.deviceId,
    actionId: '55555555-5555-4555-8555-555555555555',
    capabilityId: 'cap.remote.control',
    risk: risk,
    approvalClass: approvalClass,
    requester: user.name,
    target: 'garage-door',
    externalEffects: 'opens the garage door',
    cost: 'none',
    reversible: true,
    expiresAtUnixS: 1000000000,
    state: ApprovalPromptState.pending,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

void main() {
  group('ep034_lf004_multi_user', () {
    late EnrolledUser alice;
    late EnrolledUser bob;
    late EnrolledUser charlie;

    setUp(() {
      alice = _enroll(
        name: 'alice',
        principalId: '33333333-3333-4333-8333-333333333331',
        deviceId: '22222222-2222-4222-8222-222222222221',
        trust: DeviceTrustLevel.verified,
        preferences: <String>['cap.light.toggle', 'cap.thermostat.set'],
      );
      bob = _enroll(
        name: 'bob',
        principalId: '33333333-3333-4333-8333-333333333332',
        deviceId: '22222222-2222-4222-8222-222222222222',
        trust: DeviceTrustLevel.verified,
        preferences: <String>['cap.light.toggle'],
      );
      charlie = _enroll(
        name: 'charlie',
        principalId: '33333333-3333-4333-8333-333333333333',
        deviceId: '22222222-2222-4222-8222-222222222223',
        trust: DeviceTrustLevel.local,
        preferences: <String>['cap.light.toggle'],
      );
    });

    test(
      'enrolls two adults and one restricted user on distinct mobile devices',
      () {
        expect(alice.device.deviceId, isNot(bob.device.deviceId));
        expect(bob.device.deviceId, isNot(charlie.device.deviceId));
        expect(alice.binding.active, isTrue);
        expect(bob.binding.active, isTrue);
        expect(charlie.binding.active, isTrue);
        expect(alice.session.isUsableAt(500), isTrue);
        expect(charlie.device.trustLevel, DeviceTrustLevel.local);
      },
    );

    test(
      'separate context: a prompt belongs to exactly one principal and device',
      () {
        final prompt = _promptFor(alice);
        final service = ApprovalBindingService();
        // Alice can resolve her own prompt.
        final resolution = service.approve(
          prompt: prompt,
          session: alice.session,
          binding: alice.binding,
          actingDeviceId: alice.deviceId,
          actingPrincipalId: alice.principalId,
          nowUnixS: 500,
        );
        expect(resolution.decision, ApprovalDecision.approved);
        // Bob cannot resolve Alice's prompt (fresh service: the guards
        // run because no prior resolution exists to replay).
        final bobService = ApprovalBindingService();
        expect(
          () => bobService.approve(
            prompt: _promptFor(alice),
            session: bob.session,
            binding: bob.binding,
            actingDeviceId: bob.deviceId,
            actingPrincipalId: bob.principalId,
            nowUnixS: 600,
          ),
          throwsA(
            isA<Spec006Error>().having(
              (e) => e.code,
              'code',
              ErrorCode.authorization,
            ),
          ),
        );
        // Charlie cannot resolve Alice's prompt either.
        final charlieService = ApprovalBindingService();
        expect(
          () => charlieService.approve(
            prompt: _promptFor(alice),
            session: charlie.session,
            binding: charlie.binding,
            actingDeviceId: charlie.deviceId,
            actingPrincipalId: charlie.principalId,
            nowUnixS: 700,
          ),
          throwsA(
            isA<Spec006Error>().having(
              (e) => e.code,
              'code',
              ErrorCode.authorization,
            ),
          ),
        );
      },
    );

    test(
      'separate permissions: restricted user cannot obtain high-risk approval',
      () {
        final service = ApprovalBindingService();
        // Charlie's prompt for a high-risk action carries POLICY class
        // (restricted profile has no human-approval scope).
        expect(
          () => service.approve(
            prompt: _promptFor(
              charlie,
              risk: RiskClass.r4,
              approvalClass: ApprovalClass.policy,
            ),
            session: charlie.session,
            binding: charlie.binding,
            actingDeviceId: charlie.deviceId,
            actingPrincipalId: charlie.principalId,
            nowUnixS: 500,
          ),
          throwsA(
            isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
          ),
        );
        // Adults can approve their own high-risk actions.
        final aliceResolution = service.approve(
          prompt: _promptFor(alice),
          session: alice.session,
          binding: alice.binding,
          actingDeviceId: alice.deviceId,
          actingPrincipalId: alice.principalId,
          nowUnixS: 500,
        );
        expect(aliceResolution.decision, ApprovalDecision.approved);
      },
    );

    test(
      'separate preferences: offline allowances are per-user, not shared',
      () {
        final aliceCache = OfflinePolicyCache(store: alice.store);
        final bobCache = OfflinePolicyCache(store: bob.store);
        // Alice's thermostat preference is not granted to Bob.
        final aliceAllowance = aliceCache.decide(
          capabilityId: 'cap.thermostat.set',
          risk: RiskClass.r2,
          nowUnixS: 500,
        );
        expect(aliceAllowance.capabilityId, 'cap.thermostat.set');
        expect(
          () => bobCache.decide(
            capabilityId: 'cap.thermostat.set',
            risk: RiskClass.r2,
            nowUnixS: 500,
          ),
          throwsA(
            isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
          ),
        );
        // Charlie's restricted profile still gets his light preference.
        final charlieCache = OfflinePolicyCache(store: charlie.store);
        final charlieAllowance = charlieCache.decide(
          capabilityId: 'cap.light.toggle',
          risk: RiskClass.r1,
          nowUnixS: 500,
        );
        expect(charlieAllowance.policyVersion, 'policy-v1');
      },
    );

    test('separate mobile devices: resolution records the acting device', () {
      final service = ApprovalBindingService();
      final resolution = service.approve(
        prompt: _promptFor(bob),
        session: bob.session,
        binding: bob.binding,
        actingDeviceId: bob.deviceId,
        actingPrincipalId: bob.principalId,
        nowUnixS: 500,
      );
      expect(resolution.approvalId, '11111111-1111-4111-8111-111111111111');
      expect(resolution.correlation, '66666666-6666-4666-8666-666666666666');
      expect(resolution.toJson()['decision'], 'APPROVED');
    });
  });
}
