/// EP-034 M1 DeviceEnrollment contract (SPEC-017).
///
/// Binds the canonical `schemas/auth/passkey-challenge.schema.json`
/// and `schemas/auth/step-up-challenge.schema.json` vocabulary.
/// Passkeys and biometrics are native-module capabilities; the
/// contract layer defines the enrollment lifecycle and the step-up
/// challenge used for high-risk mobile approvals (LF-022).
library;

import 'errors.dart';
import 'validate.dart';
import 'approvals.dart' show RiskClass;

/// Canonical passkey challenge states (passkey-challenge schema enum).
enum PasskeyChallengeState {
  pendingChallenge('PENDING_CHALLENGE'),
  registered('REGISTERED'),
  revoked('REVOKED'),
  expired('EXPIRED'),
  failed('FAILED');

  const PasskeyChallengeState(this.wire);
  final String wire;
}

/// Canonical step-up challenge states (step-up-challenge schema enum).
enum StepUpChallengeState {
  pending('PENDING'),
  completed('COMPLETED'),
  failed('FAILED'),
  expired('EXPIRED'),
  cancelled('CANCELLED');

  const StepUpChallengeState(this.wire);
  final String wire;
}

/// Required challenge strengths (step-up-challenge schema enum).
enum ChallengeStrength {
  none('NONE'),
  singleFactor('SINGLE_FACTOR'),
  multiFactor('MULTI_FACTOR'),
  stepUp('STEP_UP');

  const ChallengeStrength(this.wire);
  final String wire;
}

/// PasskeyChallenge: enrollment of a device-bound passkey (SPEC-017
/// behavior 2: native modules implement passkeys and biometrics).
class PasskeyChallenge {
  const PasskeyChallenge({
    required this.challengeId,
    required this.tenantId,
    required this.principalId,
    required this.deviceId,
    required this.challenge,
    required this.createdAtUnixS,
    required this.expiresAtUnixS,
    required this.correlation,
    required this.state,
  });

  factory PasskeyChallenge.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'challenge_id',
      'tenant_id',
      'principal_id',
      'device_id',
      'challenge',
      'created_at_unix_s',
      'expires_at_unix_s',
      'correlation',
      'state',
    };
    rejectUnknownKeys(json, allowed);
    final createdAt = json['created_at_unix_s'];
    final expiresAt = json['expires_at_unix_s'];
    return PasskeyChallenge(
      challengeId: requireUuid(json, 'challenge_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      deviceId: requireUuid(json, 'device_id'),
      challenge: requireString(json, 'challenge'),
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
      state: requireEnum(
        json,
        'state',
        PasskeyChallengeState.values,
        wireOf: (v) => v.wire,
      ),
    );
  }

  final String challengeId;
  final String tenantId;
  final String principalId;
  final String deviceId;
  final String challenge;
  final int createdAtUnixS;
  final int expiresAtUnixS;
  final String correlation;
  final PasskeyChallengeState state;

  bool isExpiredAt(int nowUnixS) => nowUnixS > expiresAtUnixS;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'challenge_id': challengeId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'device_id': deviceId,
    'challenge': challenge,
    'created_at_unix_s': createdAtUnixS,
    'expires_at_unix_s': expiresAtUnixS,
    'correlation': correlation,
    'state': state.wire,
  };
}

/// StepUpChallenge: the mobile step-up gate for high-risk actions
/// (SPEC-017 behavior 2; LF-022 mobile-step-up). Voice-only
/// authorization is refused; mobile biometric + passkey approval is
/// required for the step-up class.
class StepUpChallenge {
  const StepUpChallenge({
    required this.challengeId,
    required this.tenantId,
    required this.principalId,
    required this.risk,
    required this.requiredStrength,
    required this.challenge,
    required this.createdAtUnixS,
    required this.expiresAtUnixS,
    required this.correlation,
    required this.state,
  });

  factory StepUpChallenge.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'challenge_id',
      'tenant_id',
      'principal_id',
      'risk',
      'required_strength',
      'challenge',
      'created_at_unix_s',
      'expires_at_unix_s',
      'correlation',
      'state',
    };
    rejectUnknownKeys(json, allowed);
    final createdAt = json['created_at_unix_s'];
    final expiresAt = json['expires_at_unix_s'];
    return StepUpChallenge(
      challengeId: requireUuid(json, 'challenge_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      risk: requireEnum(json, 'risk', RiskClass.values, wireOf: (v) => v.wire),
      requiredStrength: requireEnum(
        json,
        'required_strength',
        ChallengeStrength.values,
        wireOf: (v) => v.wire,
      ),
      challenge: requireString(json, 'challenge'),
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
      state: requireEnum(
        json,
        'state',
        StepUpChallengeState.values,
        wireOf: (v) => v.wire,
      ),
    );
  }

  final String challengeId;
  final String tenantId;
  final String principalId;
  final RiskClass risk;
  final ChallengeStrength requiredStrength;
  final String challenge;
  final int createdAtUnixS;
  final int expiresAtUnixS;
  final String correlation;
  final StepUpChallengeState state;

  bool isExpiredAt(int nowUnixS) => nowUnixS > expiresAtUnixS;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'challenge_id': challengeId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'risk': risk.wire,
    'required_strength': requiredStrength.wire,
    'challenge': challenge,
    'created_at_unix_s': createdAtUnixS,
    'expires_at_unix_s': expiresAtUnixS,
    'correlation': correlation,
    'state': state.wire,
  };
}
