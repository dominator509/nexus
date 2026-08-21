/// EP-034 M1 VoiceRemote and BackgroundVoice contracts (SPEC-017).
///
/// Voice remote control uses the same server capability and policy
/// path as web/desktop; there is no hidden mobile bypass (SPEC-017
/// behavior 5). Voice is an input channel, never an authority:
/// voice-only authorization is refused for step-up approvals.
library;

import 'validate.dart';

/// Canonical background voice states.
enum BackgroundVoiceState {
  idle('IDLE'),
  listening('LISTENING'),
  processing('PROCESSING'),
  speaking('SPEAKING'),
  error('ERROR');

  const BackgroundVoiceState(this.wire);
  final String wire;
}

/// Canonical voice command states.
enum VoiceCommandState {
  received('RECEIVED'),
  evaluated('EVALUATED'),
  denied('DENIED'),
  awaitingApproval('AWAITING_APPROVAL'),
  executing('EXECUTING'),
  succeeded('SUCCEEDED'),
  failed('FAILED');

  const VoiceCommandState(this.wire);
  final String wire;
}

/// BackgroundVoice: platform background audio/voice capability
/// (SPEC-017 behavior 1/2). The contract layer is provider-neutral;
/// native modules own the audio path.
class BackgroundVoice {
  const BackgroundVoice({
    required this.deviceId,
    required this.enabled,
    required this.state,
    this.errorCode,
  });

  factory BackgroundVoice.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{'device_id', 'enabled', 'state', 'error_code'};
    rejectUnknownKeys(json, allowed);
    final enabled = json['enabled'];
    final error = json['error_code'];
    return BackgroundVoice(
      deviceId: requireUuid(json, 'device_id'),
      enabled: enabled is bool ? enabled : false,
      state: requireEnum(
        json,
        'state',
        BackgroundVoiceState.values,
        wireOf: (v) => v.wire,
      ),
      errorCode: error is String ? error : null,
    );
  }

  final String deviceId;
  final bool enabled;
  final BackgroundVoiceState state;
  final String? errorCode;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'device_id': deviceId,
    'enabled': enabled,
    'state': state.wire,
    if (errorCode != null) 'error_code': errorCode,
  };
}

/// VoiceRemote: a voice-initiated command surface (SPEC-017 behavior
/// 5). The command carries the canonical capability id and
/// idempotency key; text is data, never authority.
class VoiceRemote {
  const VoiceRemote({
    required this.commandId,
    required this.tenantId,
    required this.principalId,
    required this.deviceId,
    required this.capabilityId,
    required this.transcript,
    required this.idempotencyKey,
    required this.state,
    required this.correlation,
  });

  factory VoiceRemote.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'command_id',
      'tenant_id',
      'principal_id',
      'device_id',
      'capability_id',
      'transcript',
      'idempotency_key',
      'state',
      'correlation',
    };
    rejectUnknownKeys(json, allowed);
    return VoiceRemote(
      commandId: requireUuid(json, 'command_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      deviceId: requireUuid(json, 'device_id'),
      capabilityId: requireString(json, 'capability_id'),
      transcript: requireString(json, 'transcript'),
      idempotencyKey: requireIdempotencyKey(json, 'idempotency_key'),
      state: requireEnum(
        json,
        'state',
        VoiceCommandState.values,
        wireOf: (v) => v.wire,
      ),
      correlation: requireUuid(json, 'correlation'),
    );
  }

  final String commandId;
  final String tenantId;
  final String principalId;
  final String deviceId;
  final String capabilityId;
  final String transcript;
  final String idempotencyKey;
  final VoiceCommandState state;
  final String correlation;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'command_id': commandId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'device_id': deviceId,
    'capability_id': capabilityId,
    'transcript': transcript,
    'idempotency_key': idempotencyKey,
    'state': state.wire,
    'correlation': correlation,
  };
}
