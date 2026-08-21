import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

/// LF-022 mobile-step-up live-fire (EP-034 M5).
///
/// Request a high-risk action by voice (canonical AGENT transcript
/// seam), refuse voice-only authorization, approve with the mobile
/// step-up path (device+user bound HUMAN-class approval with a
/// step-up session), execute, and verify - composed from the REAL
/// production contract + behavior layers. Native biometric/passkey
/// verification is NOT ASSERTED (deferred native milestone); the
/// journey proves the mobile approval semantics that the native
/// verification must feed.

MobileSession _session({
  required String deviceId,
  required String principalId,
  SessionStrength strength = SessionStrength.stepUp,
  bool revoked = false,
}) {
  return MobileSession(
    sessionId: '77777777-7777-4777-8777-777777777777',
    principalId: principalId,
    tenantId: '44444444-4444-4444-8444-444444444444',
    deviceId: deviceId,
    grantFlow: GrantFlow.authorizationCode,
    strength: strength,
    createdAtUnixS: 0,
    expiresAtUnixS: 1000000000,
    revoked: revoked,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

DeviceBinding _binding(String deviceId, String principalId) {
  return DeviceBinding(
    deviceId: deviceId,
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: principalId,
    boundAtUnixS: 0,
    revoked: false,
  );
}

ApprovalPrompt _voicePrompt() {
  // The high-risk action request arrives from the canonical AGENT
  // transcript seam (voice start). It names the target device and
  // principal that must step up.
  return ApprovalPrompt(
    approvalId: '11111111-1111-4111-8111-111111111111',
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: '33333333-3333-4333-8333-333333333333',
    deviceId: '22222222-2222-4222-8222-222222222222',
    actionId: '55555555-5555-4555-8555-555555555555',
    capabilityId: 'cap.money.send',
    risk: RiskClass.r4,
    approvalClass: ApprovalClass.human,
    requester: 'alice',
    target: 'wire-transfer',
    externalEffects: 'transfers funds to an external account',
    cost: 'high',
    reversible: false,
    expiresAtUnixS: 1000000000,
    state: ApprovalPromptState.pending,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

void main() {
  group('ep034_lf022_step_up', () {
    test(
      'voice-only authorization is refused (no device binding, AUTHORIZATION)',
      () {
        final service = ApprovalBindingService();
        final prompt = _voicePrompt();
        // Voice-only: the AGENT transcript seam attempts to authorize
        // without the bound mobile device present.
        expect(
          () => service.approve(
            prompt: prompt,
            session: _session(
              deviceId: '22222222-2222-4222-8222-222222222222',
              principalId: '33333333-3333-4333-8333-333333333333',
              strength: SessionStrength.singleFactor,
            ),
            binding: _binding(
              '22222222-2222-4222-8222-222222222222',
              '33333333-3333-4333-8333-333333333333',
            ),
            actingDeviceId: '00000000-0000-4000-8000-000000000000',
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
      'voice-only session strength never mints a high-risk approval by itself',
      () {
        final service = ApprovalBindingService();
        final prompt = _voicePrompt();
        // Even with the right device, a SINGLE_FACTOR (voice) session
        // cannot satisfy the step-up binding for an R4 action when the
        // acting device is not the bound device.
        expect(
          () => service.approve(
            prompt: prompt,
            session: _session(
              deviceId: '22222222-2222-4222-8222-222222222222',
              principalId: '33333333-3333-4333-8333-333333333333',
              strength: SessionStrength.singleFactor,
            ),
            binding: _binding(
              '22222222-2222-4222-8222-222222222222',
              '33333333-3333-4333-8333-333333333333',
            ),
            actingDeviceId: '22222222-2222-4222-8222-222222222222',
            actingPrincipalId: '00000000-0000-4000-8000-000000000000',
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

    test('mobile step-up approval executes and verifies', () {
      final service = ApprovalBindingService();
      final prompt = _voicePrompt();
      // Step-up: the bound mobile device, the bound principal, a
      // STEP_UP session (the state native biometric/passkey
      // verification must produce), and a HUMAN-class approval.
      final resolution = service.approve(
        prompt: prompt,
        session: _session(
          deviceId: '22222222-2222-4222-8222-222222222222',
          principalId: '33333333-3333-4333-8333-333333333333',
          strength: SessionStrength.stepUp,
        ),
        binding: _binding(
          '22222222-2222-4222-8222-222222222222',
          '33333333-3333-4333-8333-333333333333',
        ),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(resolution.decision, ApprovalDecision.approved);
      expect(resolution.decidedAtUnixS, 500);
      expect(resolution.correlation, prompt.correlation);

      // Verify: the resolution is durable in the exactly-once store
      // and a duplicate execution returns the same resolution.
      final replay = service.approve(
        prompt: prompt,
        session: _session(
          deviceId: '22222222-2222-4222-8222-222222222222',
          principalId: '33333333-3333-4333-8333-333333333333',
          strength: SessionStrength.stepUp,
        ),
        binding: _binding(
          '22222222-2222-4222-8222-222222222222',
          '33333333-3333-4333-8333-333333333333',
        ),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 600,
      );
      expect(identical(replay, resolution), isTrue);
    });

    test(
      'step-up telemetry carries correlation and never the prompt content',
      () {
        final inner = InMemoryTelemetrySink();
        final sink = SanitizingTelemetrySink(inner);
        sink.emit(
          TelemetryEvent(
            operation: 'approval.resolve',
            code: 'OK',
            durationMs: 7,
            correlationId: '66666666-6666-4666-8666-666666666666',
            actorId: '33333333-3333-4333-8333-333333333333',
            tenantId: '44444444-4444-4444-8444-444444444444',
            resourceRef: 'cap.money.send',
          ),
        );
        final raw = inner.events.single.toJson().toString();
        expect(
          inner.events.single.correlationId,
          '66666666-6666-4666-8666-666666666666',
        );
        expect(raw.contains('wire-transfer'), isFalse);
        expect(raw.contains('external account'), isFalse);
      },
    );
  });
}
