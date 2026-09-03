import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

ApprovalPrompt _prompt({
  String approvalId = '11111111-1111-4111-8111-111111111111',
  String deviceId = '22222222-2222-4222-8222-222222222222',
  String principalId = '33333333-3333-4333-8333-333333333333',
  RiskClass risk = RiskClass.r4,
  ApprovalClass approvalClass = ApprovalClass.human,
  ApprovalPromptState state = ApprovalPromptState.pending,
  int expiresAtUnixS = 1000,
}) {
  return ApprovalPrompt(
    approvalId: approvalId,
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

MobileSession _session({
  bool revoked = false,
  int expiresAtUnixS = 1000,
  SessionStrength strength = SessionStrength.stepUp,
  String? principalId,
  String? deviceId,
  String? tenantId,
}) {
  return MobileSession(
    sessionId: '77777777-7777-4777-8777-777777777777',
    principalId: principalId ?? '33333333-3333-4333-8333-333333333333',
    tenantId: tenantId ?? '44444444-4444-4444-8444-444444444444',
    deviceId: deviceId ?? '22222222-2222-4222-8222-222222222222',
    grantFlow: GrantFlow.authorizationCode,
    strength: strength,
    createdAtUnixS: 0,
    expiresAtUnixS: expiresAtUnixS,
    revoked: revoked,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

DeviceBinding _binding({
  bool revoked = false,
  String? deviceId,
  String? principalId,
  String? tenantId,
}) {
  return DeviceBinding(
    deviceId: deviceId ?? '22222222-2222-4222-8222-222222222222',
    tenantId: tenantId ?? '44444444-4444-4444-8444-444444444444',
    principalId: principalId ?? '33333333-3333-4333-8333-333333333333',
    boundAtUnixS: 0,
    revoked: revoked,
  );
}

void main() {
  group('ep034_unit_approval_binding', () {
    test(
      'high-risk approval binds to device: wrong acting device is AUTHORIZATION',
      () {
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
      'high-risk approval binds to user: wrong acting principal is AUTHORIZATION',
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

    test('revoked device binding refuses high-risk approval', () {
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
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.authorization,
          ),
        ),
      );
    });

    test('expired approval prompt is not actionable (POLICY)', () {
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
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test('boundary: prompt actionable exactly at expiration instant', () {
      final service = ApprovalBindingService();
      final resolution = service.approve(
        prompt: _prompt(expiresAtUnixS: 500),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(resolution.decision, ApprovalDecision.approved);
    });

    test('revoked session refuses approval (AUTHORIZATION)', () {
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
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.authorization,
          ),
        ),
      );
    });

    test('R3 approval with POLICY class is refused (POLICY)', () {
      final service = ApprovalBindingService();
      expect(
        () => service.approve(
          prompt: _prompt(
            risk: RiskClass.r3,
            approvalClass: ApprovalClass.policy,
          ),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test('R4 approval with HUMAN class is accepted', () {
      final service = ApprovalBindingService();
      final resolution = service.approve(
        prompt: _prompt(risk: RiskClass.r4, approvalClass: ApprovalClass.human),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(resolution.decision, ApprovalDecision.approved);
      expect(resolution.decidedAtUnixS, 500);
    });

    test('FOUR_EYES class satisfies the high-risk binding', () {
      final service = ApprovalBindingService();
      final resolution = service.approve(
        prompt: _prompt(
          risk: RiskClass.r4,
          approvalClass: ApprovalClass.fourEyes,
        ),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(resolution.decision, ApprovalDecision.approved);
    });

    // ------------------------------------------------------------------
    // AUD-041 regressions: step-up enforcement + binding identity truth.
    // ------------------------------------------------------------------

    test(
      'AUD-041: single-factor session is refused for R4 (POLICY), not an acting-ID mismatch',
      () {
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(strength: SessionStrength.singleFactor),
            binding: _binding(),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
          ),
          throwsA(
            isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
          ),
        );
      },
    );

    test(
      'AUD-041: multi-factor (non-step-up) session is refused for R4 (POLICY)',
      () {
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(strength: SessionStrength.multiFactor),
            binding: _binding(),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
          ),
          throwsA(
            isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
          ),
        );
      },
    );

    test(
      'AUD-041: session whose principal differs from the prompt is refused (AUTHORIZATION)',
      () {
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(
              principalId: '99999999-9999-4999-8999-999999999999',
            ),
            binding: _binding(),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
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
      'AUD-041: session whose device differs from the prompt is refused (AUTHORIZATION)',
      () {
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(deviceId: '99999999-9999-4999-8999-999999999999'),
            binding: _binding(),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
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
      'AUD-041: binding that does not own the prompt device is refused (AUTHORIZATION)',
      () {
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(),
            binding: _binding(deviceId: '99999999-9999-4999-8999-999999999999'),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
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
      'AUD-041: binding that does not own the prompt principal is refused (AUTHORIZATION)',
      () {
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(),
            binding: _binding(
              principalId: '99999999-9999-4999-8999-999999999999',
            ),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
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
      'AUD-041: session whose tenant differs from the prompt is refused (AUTHORIZATION)',
      () {
        // The session must own the SAME tenant named by the approval
        // prompt - a session from a different tenant can never bind
        // this approval.
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(tenantId: '99999999-9999-4999-8999-999999999999'),
            binding: _binding(),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
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
      'AUD-041: binding that does not own the prompt tenant is refused (AUTHORIZATION)',
      () {
        // The device binding must own the SAME tenant named by the
        // approval prompt - a binding registered under a different
        // tenant can never authorize this approval.
        final service = ApprovalBindingService();
        expect(
          () => service.approve(
            prompt: _prompt(),
            session: _session(),
            binding: _binding(tenantId: '99999999-9999-4999-8999-999999999999'),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '33333333-3333-4333-8333-333333333333',
            nowUnixS: 500,
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

    test('AUD-041: NONE-strength session is refused for R4 (POLICY)', () {
      // A session that never completed any authentication cannot
      // resolve a high-risk approval.
      final service = ApprovalBindingService();
      expect(
        () => service.approve(
          prompt: _prompt(),
          session: _session(strength: SessionStrength.none),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.policy),
        ),
      );
    });

    test(
      'AUD-041: R1 approval with a single-factor session is still accepted',
      () {
        final service = ApprovalBindingService();
        final resolution = service.approve(
          prompt: _prompt(
            risk: RiskClass.r1,
            approvalClass: ApprovalClass.policy,
          ),
          session: _session(strength: SessionStrength.singleFactor),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        );
        expect(resolution.decision, ApprovalDecision.approved);
      },
    );

    test(
      'approval resolution is idempotent: duplicate approve returns the same resolution',
      () {
        final service = ApprovalBindingService();
        final first = service.approve(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        );
        final second = service.approve(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 600,
        );
        expect(identical(first, second), isTrue);
        expect(second.decidedAtUnixS, 500);
      },
    );

    test('approval resolved once cannot be re-denied (CONFLICT)', () {
      final service = ApprovalBindingService();
      service.approve(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(
        () => service.deny(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 600,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.conflict),
        ),
      );
    });

    test('denied approval cannot be re-approved (CONFLICT)', () {
      final service = ApprovalBindingService();
      service.deny(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(
        () => service.approve(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 600,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.conflict),
        ),
      );
    });

    test('resolution carries correlation and decision vocabulary', () {
      final service = ApprovalBindingService();
      final resolution = service.approve(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(resolution.correlation, '66666666-6666-4666-8666-666666666666');
      expect(resolution.toJson()['decision'], 'APPROVED');
      expect(
        resolution.toJson()['approval_id'],
        '11111111-1111-4111-8111-111111111111',
      );
    });
  });
}
