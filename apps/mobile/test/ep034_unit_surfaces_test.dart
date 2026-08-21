import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';

void main() {
  test('contract layer exposes the eight EP-034 public interfaces', () {
    // Compile-time binding: each public interface must be importable
    // from the package barrel.
    const session = MobileSession(
      sessionId: '11111111-1111-4111-8111-111111111111',
      principalId: '22222222-2222-4222-8222-222222222222',
      tenantId: '33333333-3333-4333-8333-333333333333',
      deviceId: '44444444-4444-4444-8444-444444444444',
      grantFlow: GrantFlow.authorizationCode,
      strength: SessionStrength.multiFactor,
      createdAtUnixS: 1,
      expiresAtUnixS: 2,
      revoked: false,
      correlation: '55555555-5555-4555-8555-555555555555',
    );
    expect(session.sessionId, isNotEmpty);

    const voice = VoiceRemote(
      commandId: '11111111-1111-4111-8111-111111111111',
      tenantId: '33333333-3333-4333-8333-333333333333',
      principalId: '22222222-2222-4222-8222-222222222222',
      deviceId: '44444444-4444-4444-8444-444444444444',
      capabilityId: 'capability.v1',
      transcript: 'turn off the lights',
      idempotencyKey: 'idem-00000000000001',
      state: VoiceCommandState.received,
      correlation: '55555555-5555-4555-8555-555555555555',
    );
    expect(voice.capabilityId, 'capability.v1');

    const approval = ApprovalPrompt(
      approvalId: '11111111-1111-4111-8111-111111111111',
      tenantId: '33333333-3333-4333-8333-333333333333',
      principalId: '22222222-2222-4222-8222-222222222222',
      deviceId: '44444444-4444-4444-8444-444444444444',
      actionId: '66666666-6666-4666-8666-666666666666',
      capabilityId: 'capability.v1',
      risk: RiskClass.r3,
      approvalClass: ApprovalClass.fourEyes,
      requester: 'alice',
      target: 'lights:living-room',
      externalEffects: 'none',
      cost: 'low',
      reversible: true,
      expiresAtUnixS: 2,
      state: ApprovalPromptState.pending,
      correlation: '55555555-5555-4555-8555-555555555555',
    );
    expect(approval.approvalClass, ApprovalClass.fourEyes);

    const challenge = PasskeyChallenge(
      challengeId: '11111111-1111-4111-8111-111111111111',
      tenantId: '33333333-3333-4333-8333-333333333333',
      principalId: '22222222-2222-4222-8222-222222222222',
      deviceId: '44444444-4444-4444-8444-444444444444',
      challenge: 'challenge-bytes',
      createdAtUnixS: 1,
      expiresAtUnixS: 2,
      correlation: '55555555-5555-4555-8555-555555555555',
      state: PasskeyChallengeState.pendingChallenge,
    );
    expect(challenge.challengeId, isNotEmpty);

    const endpoint = BluetoothEndpoint(
      endpointId: '11111111-1111-4111-8111-111111111111',
      localDeviceId: '44444444-4444-4444-8444-444444444444',
      kind: BluetoothEndpointKind.speaker,
      displayName: 'Living Room Speaker',
      pairingState: PairingState.notPaired,
    );
    expect(endpoint.kind, BluetoothEndpointKind.speaker);

    const store = SecureStore(
      deviceId: '44444444-4444-4444-8444-444444444444',
      tenantId: '33333333-3333-4333-8333-333333333333',
      entries: [],
    );
    expect(store.deviceId, isNotEmpty);
    expect(SecureStore.allowedOfflineKinds, isNotEmpty);

    const push = PushNotification(
      notificationId: '11111111-1111-4111-8111-111111111111',
      tenantId: '33333333-3333-4333-8333-333333333333',
      principalId: '22222222-2222-4222-8222-222222222222',
      deviceId: '44444444-4444-4444-8444-444444444444',
      kind: PushNotificationKind.approvalRequest,
      opaqueRef: 'opaque:ref:123',
      state: PushInboxState.unread,
      receivedAtUnixS: 1,
    );
    expect(push.opaqueRef, isNotEmpty);

    const remote = RemoteSession(
      sessionId: '11111111-1111-4111-8111-111111111111',
      tenantId: '33333333-3333-4333-8333-333333333333',
      principalId: '22222222-2222-4222-8222-222222222222',
      deviceId: '44444444-4444-4444-8444-444444444444',
      targetDeviceId: '77777777-7777-4777-8777-777777777777',
      state: RemoteSessionState.active,
      createdAtUnixS: 1,
      expiresAtUnixS: 2,
      correlation: '55555555-5555-4555-8555-555555555555',
    );
    expect(remote.targetDeviceId, isNotEmpty);
  });
}
