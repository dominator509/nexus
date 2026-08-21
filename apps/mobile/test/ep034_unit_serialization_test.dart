import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';

void main() {
  group('ep034_unit_serialization', () {
    Map<String, dynamic> validSessionJson() => <String, dynamic>{
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
    };

    test('MobileSession round-trips wire JSON', () {
      final parsed = MobileSession.fromJson(validSessionJson());
      expect(parsed.toJson(), validSessionJson());
      expect(parsed.strength, SessionStrength.multiFactor);
    });

    test('ClientDevice round-trips wire JSON', () {
      final json = <String, dynamic>{
        'device_id': '44444444-4444-4444-8444-444444444444',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'display_name': 'Pixel',
        'kind': 'PHONE',
        'trust_level': 'LOCAL',
      };
      final parsed = ClientDevice.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.kind, DeviceKind.phone);
      expect(parsed.trustLevel, DeviceTrustLevel.local);
    });

    test('DeviceBinding round-trips wire JSON', () {
      final json = <String, dynamic>{
        'device_id': '44444444-4444-4444-8444-444444444444',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'principal_id': '22222222-2222-4222-8222-222222222222',
        'bound_at_unix_s': 100,
        'revoked': false,
      };
      final parsed = DeviceBinding.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.active, isTrue);
    });

    test('ApprovalPrompt round-trips wire JSON with full disclosure', () {
      final json = <String, dynamic>{
        'approval_id': '11111111-1111-4111-8111-111111111111',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'principal_id': '22222222-2222-4222-8222-222222222222',
        'device_id': '44444444-4444-4444-8444-444444444444',
        'action_id': '66666666-6666-4666-8666-666666666666',
        'capability_id': 'capability.v1',
        'risk': 'R3',
        'approval_class': 'FOUR_EYES',
        'requester': 'alice',
        'target': 'lights:living-room',
        'external_effects': 'none',
        'cost': 'low',
        'reversible': true,
        'expires_at_unix_s': 200,
        'state': 'PENDING',
        'correlation': '55555555-5555-4555-8555-555555555555',
      };
      final parsed = ApprovalPrompt.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.approvalClass.requiresHuman, isTrue);
      expect(parsed.risk, RiskClass.r3);
      expect(parsed.isActionableAt(150), isTrue);
      expect(parsed.isActionableAt(201), isFalse);
    });

    test('StepUpChallenge round-trips wire JSON', () {
      final json = <String, dynamic>{
        'challenge_id': '11111111-1111-4111-8111-111111111111',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'principal_id': '22222222-2222-4222-8222-222222222222',
        'risk': 'R4',
        'required_strength': 'STEP_UP',
        'challenge': 'challenge-bytes',
        'created_at_unix_s': 1,
        'expires_at_unix_s': 2,
        'correlation': '55555555-5555-4555-8555-555555555555',
        'state': 'PENDING',
      };
      final parsed = StepUpChallenge.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.requiredStrength, ChallengeStrength.stepUp);
      expect(parsed.risk, RiskClass.r4);
    });

    test('BluetoothDiscovery round-trips wire JSON with endpoints', () {
      final json = <String, dynamic>{
        'discovery_id': '11111111-1111-4111-8111-111111111111',
        'device_id': '44444444-4444-4444-8444-444444444444',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'started_at_unix_s': 1,
        'endpoints': <Map<String, dynamic>>[
          <String, dynamic>{
            'endpoint_id': '88888888-8888-4888-8888-888888888888',
            'local_device_id': '44444444-4444-4444-8444-444444444444',
            'kind': 'SPEAKER',
            'display_name': 'Hall Speaker',
            'pairing_state': 'PAIRED',
            'rssi': -55,
          },
        ],
      };
      final parsed = BluetoothDiscovery.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.endpoints, hasLength(1));
      expect(parsed.endpoints.first.pairingState, PairingState.paired);
    });

    test('SecureStore round-trips entries', () {
      final json = <String, dynamic>{
        'device_id': '44444444-4444-4444-8444-444444444444',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'entries': <Map<String, dynamic>>[
          <String, dynamic>{
            'key': 'policy:default',
            'kind': 'CACHED_POLICY',
            'device_id': '44444444-4444-4444-8444-444444444444',
            'tenant_id': '33333333-3333-4333-8333-333333333333',
            'updated_at_unix_s': 1,
          },
        ],
      };
      final parsed = SecureStore.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.entries.first.kind, SecureStoreEntryKind.cachedPolicy);
    });

    test('PushInbox round-trips minimal opaque references', () {
      final json = <String, dynamic>{
        'device_id': '44444444-4444-4444-8444-444444444444',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'principal_id': '22222222-2222-4222-8222-222222222222',
        'notifications': <Map<String, dynamic>>[
          <String, dynamic>{
            'notification_id': '11111111-1111-4111-8111-111111111111',
            'tenant_id': '33333333-3333-4333-8333-333333333333',
            'principal_id': '22222222-2222-4222-8222-222222222222',
            'device_id': '44444444-4444-4444-8444-444444444444',
            'kind': 'APPROVAL_REQUEST',
            'opaque_ref': 'opaque:ref:42',
            'state': 'UNREAD',
            'received_at_unix_s': 1,
          },
        ],
      };
      final parsed = PushInbox.fromJson(json);
      expect(parsed.toJson(), json);
      expect(
        parsed.notifications.first.kind,
        PushNotificationKind.approvalRequest,
      );
    });

    test('RemoteControlCommand round-trips typed arguments', () {
      final json = <String, dynamic>{
        'command_id': '11111111-1111-4111-8111-111111111111',
        'session_id': '99999999-9999-4999-8999-999999999999',
        'capability_id': 'capability.v1',
        'risk': 'R2',
        'approval_class': 'HUMAN',
        'idempotency_key': 'idem-00000000000001',
        'arguments': <String, dynamic>{'target': 'lights:kitchen', 'on': true},
        'correlation': '55555555-5555-4555-8555-555555555555',
      };
      final parsed = RemoteControlCommand.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.approvalClass, ApprovalClass.human);
      expect(parsed.arguments['on'], isTrue);
    });

    test('RemoteSession round-trips and enforces expiry', () {
      final json = <String, dynamic>{
        'session_id': '11111111-1111-4111-8111-111111111111',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'principal_id': '22222222-2222-4222-8222-222222222222',
        'device_id': '44444444-4444-4444-8444-444444444444',
        'target_device_id': '77777777-7777-4777-8777-777777777777',
        'state': 'ACTIVE',
        'created_at_unix_s': 1,
        'expires_at_unix_s': 200,
        'correlation': '55555555-5555-4555-8555-555555555555',
      };
      final parsed = RemoteSession.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.isUsableAt(150), isTrue);
      expect(parsed.isUsableAt(201), isFalse);
    });

    test('BackgroundVoice round-trips wire JSON', () {
      final json = <String, dynamic>{
        'device_id': '44444444-4444-4444-8444-444444444444',
        'enabled': true,
        'state': 'LISTENING',
      };
      final parsed = BackgroundVoice.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.state, BackgroundVoiceState.listening);
    });

    test('PushEndpoint round-trips wire JSON', () {
      final json = <String, dynamic>{
        'endpoint_id': 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
        'device_id': '44444444-4444-4444-8444-444444444444',
        'tenant_id': '33333333-3333-4333-8333-333333333333',
        'provider': 'apns',
        'token': 'opaque-device-token',
      };
      final parsed = PushEndpoint.fromJson(json);
      expect(parsed.toJson(), json);
      expect(parsed.provider, 'apns');
    });
  });
}
