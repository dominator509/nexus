/// EP-034 M1 RemoteControl contract (SPEC-017).
///
/// Remote controls use the same server capability and policy path as
/// voice and web; there is no hidden mobile bypass (SPEC-017 behavior
/// 5). A RemoteSession is a device-bound, expiring session for
/// controlling other Nexus surfaces.
library;

import 'errors.dart';
import 'validate.dart';
import 'approvals.dart' show RiskClass, ApprovalClass;

/// Canonical remote session states.
enum RemoteSessionState {
  active('ACTIVE'),
  awaitingStepUp('AWAITING_STEP_UP'),
  expired('EXPIRED'),
  revoked('REVOKED');

  const RemoteSessionState(this.wire);
  final String wire;
}

/// RemoteSession: a device-bound control session. High-risk controls
/// require step-up (biometric + passkey) before consequential
/// execution (LF-022).
class RemoteSession {
  const RemoteSession({
    required this.sessionId,
    required this.tenantId,
    required this.principalId,
    required this.deviceId,
    required this.targetDeviceId,
    required this.state,
    required this.createdAtUnixS,
    required this.expiresAtUnixS,
    required this.correlation,
  });

  factory RemoteSession.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'session_id',
      'tenant_id',
      'principal_id',
      'device_id',
      'target_device_id',
      'state',
      'created_at_unix_s',
      'expires_at_unix_s',
      'correlation',
    };
    rejectUnknownKeys(json, allowed);
    final createdAt = json['created_at_unix_s'];
    final expiresAt = json['expires_at_unix_s'];
    return RemoteSession(
      sessionId: requireUuid(json, 'session_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      deviceId: requireUuid(json, 'device_id'),
      targetDeviceId: requireUuid(json, 'target_device_id'),
      state: requireEnum(
        json,
        'state',
        RemoteSessionState.values,
        wireOf: (v) => v.wire,
      ),
      createdAtUnixS: createdAt is int
          ? createdAt
          : throw Spec006Error(
              ErrorCode.validation,
              'created_at_unix_s must be an integer',
            ),
      expiresAtUnixS: expiresAt is int
          ? expiresAt
          : throw Spec006Error(
              ErrorCode.validation,
              'expires_at_unix_s must be an integer',
            ),
      correlation: requireUuid(json, 'correlation'),
    );
  }

  final String sessionId;
  final String tenantId;
  final String principalId;
  final String deviceId;
  final String targetDeviceId;
  final RemoteSessionState state;
  final int createdAtUnixS;
  final int expiresAtUnixS;
  final String correlation;

  bool isExpiredAt(int nowUnixS) => nowUnixS > expiresAtUnixS;

  bool isUsableAt(int nowUnixS) =>
      state == RemoteSessionState.active && !isExpiredAt(nowUnixS);

  Map<String, dynamic> toJson() => <String, dynamic>{
    'session_id': sessionId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'device_id': deviceId,
    'target_device_id': targetDeviceId,
    'state': state.wire,
    'created_at_unix_s': createdAtUnixS,
    'expires_at_unix_s': expiresAtUnixS,
    'correlation': correlation,
  };
}

/// RemoteControlCommand: a typed control command over a remote
/// session. Idempotency-keyed; risk-classed; the capability id is
/// vocabulary-checked before any dispatch.
class RemoteControlCommand {
  const RemoteControlCommand({
    required this.commandId,
    required this.sessionId,
    required this.capabilityId,
    required this.risk,
    required this.approvalClass,
    required this.idempotencyKey,
    required this.arguments,
    required this.correlation,
  });

  factory RemoteControlCommand.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'command_id',
      'session_id',
      'capability_id',
      'risk',
      'approval_class',
      'idempotency_key',
      'arguments',
      'correlation',
    };
    rejectUnknownKeys(json, allowed);
    final args = json['arguments'];
    return RemoteControlCommand(
      commandId: requireUuid(json, 'command_id'),
      sessionId: requireUuid(json, 'session_id'),
      capabilityId: requireString(json, 'capability_id'),
      risk: requireEnum(json, 'risk', RiskClass.values, wireOf: (v) => v.wire),
      approvalClass: requireEnum(
        json,
        'approval_class',
        ApprovalClass.values,
        wireOf: (v) => v.wire,
      ),
      idempotencyKey: requireIdempotencyKey(json, 'idempotency_key'),
      arguments: args is Map<String, dynamic>
          ? Map<String, dynamic>.unmodifiable(args)
          : throw Spec006Error(
              ErrorCode.validation,
              'arguments must be an object',
            ),
      correlation: requireUuid(json, 'correlation'),
    );
  }

  final String commandId;
  final String sessionId;
  final String capabilityId;
  final RiskClass risk;
  final ApprovalClass approvalClass;
  final String idempotencyKey;
  final Map<String, dynamic> arguments;
  final String correlation;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'command_id': commandId,
    'session_id': sessionId,
    'capability_id': capabilityId,
    'risk': risk.wire,
    'approval_class': approvalClass.wire,
    'idempotency_key': idempotencyKey,
    'arguments': arguments,
    'correlation': correlation,
  };
}
