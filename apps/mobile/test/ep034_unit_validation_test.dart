import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';

void main() {
  group('ep034_unit_validation', () {
    test('deny-unknown rejects a fabricated session field', () {
      expect(
        () => MobileSession.fromJson(<String, dynamic>{
          'session_id': '11111111-1111-4111-8111-111111111111',
          'principal_id': '22222222-2222-4222-8222-222222222222',
          'tenant_id': '33333333-3333-4333-8333-333333333333',
          'device_id': '44444444-4444-4444-8444-444444444444',
          'grant_flow': 'AUTHORIZATION_CODE',
          'strength': 'MULTI_FACTOR',
          'created_at_unix_s': 1,
          'expires_at_unix_s': 2,
          'revoked': false,
          'correlation': '55555555-5555-4555-8555-555555555555',
          'is_admin': true,
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

    test('unknown enum value is rejected with VOCABULARY never defaulted', () {
      expect(
        () => ClientDevice.fromJson(<String, dynamic>{
          'device_id': '11111111-1111-4111-8111-111111111111',
          'tenant_id': '33333333-3333-4333-8333-333333333333',
          'display_name': 'Phone',
          'kind': 'HOVERBOARD',
          'trust_level': 'VERIFIED',
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

    test('fabricated approval class cannot mint authority', () {
      expect(
        () => ApprovalPrompt.fromJson(<String, dynamic>{
          'approval_id': '11111111-1111-4111-8111-111111111111',
          'tenant_id': '33333333-3333-4333-8333-333333333333',
          'principal_id': '22222222-2222-4222-8222-222222222222',
          'device_id': '44444444-4444-4444-8444-444444444444',
          'action_id': '66666666-6666-4666-8666-666666666666',
          'capability_id': 'capability.v1',
          'risk': 'R1',
          'approval_class': 'OMNIPOTENT',
          'requester': 'alice',
          'target': 'lights',
          'external_effects': 'none',
          'cost': 'low',
          'reversible': true,
          'expires_at_unix_s': 2,
          'state': 'PENDING',
          'correlation': '55555555-5555-4555-8555-555555555555',
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

    test('undersized idempotency key is a validation failure', () {
      expect(
        () => VoiceRemote.fromJson(<String, dynamic>{
          'command_id': '11111111-1111-4111-8111-111111111111',
          'tenant_id': '33333333-3333-4333-8333-333333333333',
          'principal_id': '22222222-2222-4222-8222-222222222222',
          'device_id': '44444444-4444-4444-8444-444444444444',
          'capability_id': 'capability.v1',
          'transcript': 'turn off the lights',
          'idempotency_key': 'short',
          'state': 'RECEIVED',
          'correlation': '55555555-5555-4555-8555-555555555555',
        }),
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.validation,
          ),
        ),
      );
    });

    test('malformed uuid is a validation failure', () {
      expect(
        () => PushNotification.fromJson(<String, dynamic>{
          'notification_id': 'not-a-uuid',
          'tenant_id': '33333333-3333-4333-8333-333333333333',
          'principal_id': '22222222-2222-4222-8222-222222222222',
          'device_id': '44444444-4444-4444-8444-444444444444',
          'kind': 'EVENT',
          'opaque_ref': 'opaque:ref',
          'state': 'UNREAD',
          'received_at_unix_s': 1,
        }),
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.validation,
          ),
        ),
      );
    });

    test('problem details round-trip preserves the canonical code', () {
      const details = ProblemDetails(
        code: ErrorCode.policy,
        type: 'https://schemas.nexus.local/problems/policy',
        detail: 'approval required',
        correlationId: '55555555-5555-4555-8555-555555555555',
        status: 403,
      );
      final json = details.toJson();
      final parsed = ProblemDetails.fromJson(json);
      expect(parsed.code, ErrorCode.policy);
      expect(parsed.status, 403);
      expect(parsed.correlationId, details.correlationId);
    });

    test('session expiry is terminal for usability', () {
      final session = MobileSession(
        sessionId: '11111111-1111-4111-8111-111111111111',
        principalId: '22222222-2222-4222-8222-222222222222',
        tenantId: '33333333-3333-4333-8333-333333333333',
        deviceId: '44444444-4444-4444-8444-444444444444',
        grantFlow: GrantFlow.refreshToken,
        strength: SessionStrength.stepUp,
        createdAtUnixS: 100,
        expiresAtUnixS: 200,
        revoked: false,
        correlation: '55555555-5555-4555-8555-555555555555',
      );
      expect(session.isUsableAt(150), isTrue);
      expect(session.isUsableAt(201), isFalse);
      expect(session.isExpiredAt(201), isTrue);
    });

    test('revoked binding is terminal for high-risk approval', () {
      final binding = DeviceBinding(
        deviceId: '44444444-4444-4444-8444-444444444444',
        tenantId: '33333333-3333-4333-8333-333333333333',
        principalId: '22222222-2222-4222-8222-222222222222',
        boundAtUnixS: 1,
        revoked: true,
      );
      expect(binding.active, isFalse);
    });
  });
}
