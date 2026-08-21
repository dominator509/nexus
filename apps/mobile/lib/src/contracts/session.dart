/// EP-034 M1 MobileSession contract (SPEC-017).
///
/// Binds the canonical `schemas/auth/auth-session.schema.json`
/// vocabulary: grant flow, strength, device binding, expiry, and
/// revocation. A session that is expired or revoked is terminal for
/// consequential actions; the client never blind-replays a
/// consequential command after auth refresh.
library;

import 'errors.dart';
import 'validate.dart';

/// Canonical grant flows (auth-session schema enum).
enum GrantFlow {
  authorizationCode('AUTHORIZATION_CODE'),
  refreshToken('REFRESH_TOKEN'),
  clientCredentials('CLIENT_CREDENTIALS');

  const GrantFlow(this.wire);
  final String wire;
}

/// Canonical session strengths (auth-session schema enum).
enum SessionStrength {
  none('NONE'),
  singleFactor('SINGLE_FACTOR'),
  multiFactor('MULTI_FACTOR'),
  stepUp('STEP_UP');

  const SessionStrength(this.wire);
  final String wire;
}

/// MobileSession: a device-bound, expiring session (SPEC-017
/// behavior 3: no permanent universal credential; device-bound
/// refresh, revocation, short-lived access tokens).
class MobileSession {
  const MobileSession({
    required this.sessionId,
    required this.principalId,
    required this.tenantId,
    required this.deviceId,
    required this.grantFlow,
    required this.strength,
    required this.createdAtUnixS,
    required this.expiresAtUnixS,
    required this.revoked,
    required this.correlation,
  });

  factory MobileSession.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'session_id',
      'principal_id',
      'tenant_id',
      'device_id',
      'grant_flow',
      'strength',
      'created_at_unix_s',
      'expires_at_unix_s',
      'revoked',
      'correlation',
    };
    rejectUnknownKeys(json, allowed);
    final createdAt = json['created_at_unix_s'];
    final expiresAt = json['expires_at_unix_s'];
    final revoked = json['revoked'];
    return MobileSession(
      sessionId: requireUuid(json, 'session_id'),
      principalId: requireUuid(json, 'principal_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      deviceId: requireUuid(json, 'device_id'),
      grantFlow: requireEnum(
        json,
        'grant_flow',
        GrantFlow.values,
        wireOf: (v) => v.wire,
      ),
      strength: requireEnum(
        json,
        'strength',
        SessionStrength.values,
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
      revoked: revoked is bool ? revoked : false,
      correlation: requireUuid(json, 'correlation'),
    );
  }

  final String sessionId;
  final String principalId;
  final String tenantId;
  final String deviceId;
  final GrantFlow grantFlow;
  final SessionStrength strength;
  final int createdAtUnixS;
  final int expiresAtUnixS;
  final bool revoked;
  final String correlation;

  bool isExpiredAt(int nowUnixS) => nowUnixS > expiresAtUnixS;

  bool isUsableAt(int nowUnixS) => !revoked && !isExpiredAt(nowUnixS);

  Map<String, dynamic> toJson() => <String, dynamic>{
    'session_id': sessionId,
    'principal_id': principalId,
    'tenant_id': tenantId,
    'device_id': deviceId,
    'grant_flow': grantFlow.wire,
    'strength': strength.wire,
    'created_at_unix_s': createdAtUnixS,
    'expires_at_unix_s': expiresAtUnixS,
    'revoked': revoked,
    'correlation': correlation,
  };
}

/// MobileSessionStatus: truthful client-side session state. A UI may
/// display a session that is no longer current; consequential actions
/// must re-evaluate against the backend, never trust the display.
enum MobileSessionStatus {
  active('ACTIVE'),
  expired('EXPIRED'),
  revoked('REVOKED'),
  requiresRefresh('REQUIRES_REFRESH');

  const MobileSessionStatus(this.wire);
  final String wire;
}
