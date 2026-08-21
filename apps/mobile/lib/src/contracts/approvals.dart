/// EP-034 M1 ApprovalPrompt contract (SPEC-017 / SPEC-006).
///
/// Mobile approval displays the exact action, target, risk, external
/// effects, cost, reversibility, requester, and expiration (SPEC-017
/// behavior 4). Approval class is preserved verbatim (never
/// boolean-collapsed); a prompt is display data, never authority.
library;

import 'errors.dart';
import 'validate.dart';

/// Canonical approval classes (hydra action-request schema enum).
enum ApprovalClass {
  none('NONE'),
  policy('POLICY'),
  human('HUMAN'),
  strongHuman('STRONG_HUMAN'),
  fourEyes('FOUR_EYES');

  const ApprovalClass(this.wire);
  final String wire;

  /// High-risk (R3/R4) approvals must be HUMAN or stronger.
  bool get requiresHuman =>
      this == ApprovalClass.human ||
      this == ApprovalClass.strongHuman ||
      this == ApprovalClass.fourEyes;
}

/// Canonical risk classes (action-request schema enum).
enum RiskClass {
  r0('R0'),
  r1('R1'),
  r2('R2'),
  r3('R3'),
  r4('R4');

  const RiskClass(this.wire);
  final String wire;
}

/// Canonical prompt states.
enum ApprovalPromptState {
  pending('PENDING'),
  approved('APPROVED'),
  denied('DENIED'),
  expired('EXPIRED'),
  revoked('REVOKED'),
  cancelled('CANCELLED');

  const ApprovalPromptState(this.wire);
  final String wire;
}

/// ApprovalPrompt: what the mobile approval UI must display for a
/// high-risk action (SPEC-017 behavior 4). Content is data; the
/// backend remains the authority.
class ApprovalPrompt {
  const ApprovalPrompt({
    required this.approvalId,
    required this.tenantId,
    required this.principalId,
    required this.deviceId,
    required this.actionId,
    required this.capabilityId,
    required this.risk,
    required this.approvalClass,
    required this.requester,
    required this.target,
    required this.externalEffects,
    required this.cost,
    required this.reversible,
    required this.expiresAtUnixS,
    required this.state,
    required this.correlation,
  });

  factory ApprovalPrompt.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'approval_id',
      'tenant_id',
      'principal_id',
      'device_id',
      'action_id',
      'capability_id',
      'risk',
      'approval_class',
      'requester',
      'target',
      'external_effects',
      'cost',
      'reversible',
      'expires_at_unix_s',
      'state',
      'correlation',
    };
    rejectUnknownKeys(json, allowed);
    final expiresAt = json['expires_at_unix_s'];
    final reversible = json['reversible'];
    final effects = json['external_effects'];
    final cost = json['cost'];
    return ApprovalPrompt(
      approvalId: requireUuid(json, 'approval_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      deviceId: requireUuid(json, 'device_id'),
      actionId: requireUuid(json, 'action_id'),
      capabilityId: requireString(json, 'capability_id'),
      risk: requireEnum(json, 'risk', RiskClass.values, wireOf: (v) => v.wire),
      approvalClass: requireEnum(
        json,
        'approval_class',
        ApprovalClass.values,
        wireOf: (v) => v.wire,
      ),
      requester: requireString(json, 'requester'),
      target: requireString(json, 'target'),
      externalEffects: effects is String
          ? effects
          : throw Spec006Error(
              ErrorCode.validation,
              'external_effects must be a string',
            ),
      cost: cost is String
          ? cost
          : throw Spec006Error(ErrorCode.validation, 'cost must be a string'),
      reversible: reversible is bool ? reversible : false,
      expiresAtUnixS: expiresAt is int
          ? expiresAt
          : throw Spec006Error(
              ErrorCode.validation,
              'expires_at_unix_s must be an integer',
            ),
      state: requireEnum(
        json,
        'state',
        ApprovalPromptState.values,
        wireOf: (v) => v.wire,
      ),
      correlation: requireUuid(json, 'correlation'),
    );
  }

  final String approvalId;
  final String tenantId;
  final String principalId;
  final String deviceId;
  final String actionId;
  final String capabilityId;
  final RiskClass risk;
  final ApprovalClass approvalClass;
  final String requester;
  final String target;
  final String externalEffects;
  final String cost;
  final bool reversible;
  final int expiresAtUnixS;
  final ApprovalPromptState state;
  final String correlation;

  bool isExpiredAt(int nowUnixS) => nowUnixS > expiresAtUnixS;

  /// A prompt can only be acted on while pending and unexpired.
  bool isActionableAt(int nowUnixS) =>
      state == ApprovalPromptState.pending && !isExpiredAt(nowUnixS);

  Map<String, dynamic> toJson() => <String, dynamic>{
    'approval_id': approvalId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'device_id': deviceId,
    'action_id': actionId,
    'capability_id': capabilityId,
    'risk': risk.wire,
    'approval_class': approvalClass.wire,
    'requester': requester,
    'target': target,
    'external_effects': externalEffects,
    'cost': cost,
    'reversible': reversible,
    'expires_at_unix_s': expiresAtUnixS,
    'state': state.wire,
    'correlation': correlation,
  };
}
