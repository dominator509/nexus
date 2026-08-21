import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';

void main() {
  group('ep034_failure_malformed', () {
    final valid = <String, dynamic>{
      'approval_id': '11111111-1111-4111-8111-111111111111',
      'tenant_id': '44444444-4444-4444-8444-444444444444',
      'principal_id': '33333333-3333-4333-8333-333333333333',
      'device_id': '22222222-2222-4222-8222-222222222222',
      'action_id': '55555555-5555-4555-8555-555555555555',
      'capability_id': 'cap.remote.control',
      'risk': 'R4',
      'approval_class': 'HUMAN',
      'requester': 'alice',
      'target': 'garage-door',
      'external_effects': 'opens the garage door',
      'cost': 'none',
      'reversible': true,
      'expires_at_unix_s': 1000000000,
      'state': 'PENDING',
      'correlation': '66666666-6666-4666-8666-666666666666',
    };

    test(
      'unknown field in approval wire input is rejected with VOCABULARY',
      () {
        final corrupted = Map<String, dynamic>.from(valid);
        corrupted['fabricated_capability'] = 'cap.money.send';
        expect(
          () => ApprovalPrompt.fromJson(corrupted),
          throwsA(
            isA<Spec006Error>().having(
              (e) => e.code,
              'code',
              ErrorCode.vocabulary,
            ),
          ),
        );
      },
    );

    test('fabricated approval class enum is rejected with VOCABULARY', () {
      final corrupted = Map<String, dynamic>.from(valid);
      corrupted['approval_class'] = 'OMNIPOTENT';
      expect(
        () => ApprovalPrompt.fromJson(corrupted),
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.vocabulary,
          ),
        ),
      );
    });

    test('missing required value is a VALIDATION failure', () {
      final corrupted = Map<String, dynamic>.from(valid)..remove('target');
      expect(
        () => ApprovalPrompt.fromJson(corrupted),
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.validation,
          ),
        ),
      );
    });

    test('invalid identifier (bad uuid) is a VALIDATION failure', () {
      final corrupted = Map<String, dynamic>.from(valid);
      corrupted['approval_id'] = 'not-a-uuid';
      expect(
        () => ApprovalPrompt.fromJson(corrupted),
        throwsA(
          isA<Spec006Error>().having(
            (e) => e.code,
            'code',
            ErrorCode.validation,
          ),
        ),
      );
    });

    test('unknown problem-details code is rejected with VOCABULARY', () {
      expect(
        () => ProblemDetails.fromJson(<String, dynamic>{
          'code': 'FABRICATED_CODE',
          'type': '',
          'detail': '',
          'status': 500,
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

    test('fabricated session field is rejected with VOCABULARY', () {
      expect(
        () => MobileSession.fromJson(<String, dynamic>{
          'session_id': '77777777-7777-4777-8777-777777777777',
          'principal_id': '33333333-3333-4333-8333-333333333333',
          'tenant_id': '44444444-4444-4444-8444-444444444444',
          'device_id': '22222222-2222-4222-8222-222222222222',
          'grant_flow': 'AUTHORIZATION_CODE',
          'strength': 'MULTI_FACTOR',
          'created_at_unix_s': 0,
          'expires_at_unix_s': 1000000000,
          'revoked': false,
          'correlation': '66666666-6666-4666-8666-666666666666',
          'admin_override': true,
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

    test('fabricated device trust level is rejected with VOCABULARY', () {
      expect(
        () => ClientDevice.fromJson(<String, dynamic>{
          'device_id': '22222222-2222-4222-8222-222222222222',
          'tenant_id': '44444444-4444-4444-8444-444444444444',
          'display_name': 'alice-phone',
          'kind': 'PHONE',
          'trust_level': 'ABSOLUTE',
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
  });
}
