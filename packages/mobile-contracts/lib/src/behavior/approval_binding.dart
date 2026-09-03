/// EP-034 M2 approval binding behavior (SPEC-017 behavior 4; node
/// contract: high-risk approvals bind to device AND user).
///
/// An approval prompt is display data, never authority. Resolving an
/// approval requires a usable session, an active device binding, the
/// acting device matching the prompt device, the acting principal
/// matching the prompt principal, and a risk-appropriate approval
/// class. Failures are typed SPEC-006 errors. Resolutions are
/// idempotent: the same approval resolves exactly once, and a
/// divergent re-resolution is a CONFLICT.
library;

import 'package:nexus_mobile/nexus_mobile.dart';

/// Canonical approval decision (action-request schema APPROVED/DENIED
/// vocabulary; same wire values as ApprovalPromptState).
enum ApprovalDecision {
  approved('APPROVED'),
  denied('DENIED');

  const ApprovalDecision(this.wire);
  final String wire;
}

/// Immutable outcome of binding an approval to its device and user.
class ApprovalResolution {
  const ApprovalResolution({
    required this.approvalId,
    required this.decision,
    required this.decidedAtUnixS,
    required this.correlation,
  });

  final String approvalId;
  final ApprovalDecision decision;
  final int decidedAtUnixS;
  final String correlation;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'approval_id': approvalId,
    'decision': decision.wire,
    'decided_at_unix_s': decidedAtUnixS,
    'correlation': correlation,
  };
}

/// Port for durable approval-resolution storage. The M2 in-memory
/// implementation is the real M2 layer for deterministic invariants;
/// platform persistence is a later milestone.
abstract class ApprovalResolutionStore {
  ApprovalResolution? find(String approvalId);
  void put(ApprovalResolution resolution);
}

/// Bounded in-memory resolution ring (exactly-once semantics).
class InMemoryApprovalResolutionStore implements ApprovalResolutionStore {
  InMemoryApprovalResolutionStore({this.capacity = 256});

  final int capacity;
  final Map<String, ApprovalResolution> _ring = <String, ApprovalResolution>{};
  final List<String> _order = <String>[];

  @override
  ApprovalResolution? find(String approvalId) => _ring[approvalId];

  @override
  void put(ApprovalResolution resolution) {
    if (!_ring.containsKey(resolution.approvalId)) {
      _order.add(resolution.approvalId);
      if (_order.length > capacity) {
        _ring.remove(_order.removeAt(0));
      }
    }
    _ring[resolution.approvalId] = resolution;
  }
}

/// Decides whether a high-risk approval may be resolved from a given
/// device and session. Pure domain logic; no I/O.
class ApprovalBindingService {
  ApprovalBindingService({ApprovalResolutionStore? store})
    : _store = store ?? InMemoryApprovalResolutionStore();

  final ApprovalResolutionStore _store;

  void _guard(
    ApprovalPrompt prompt,
    MobileSession session,
    DeviceBinding binding,
    String actingDeviceId,
    String actingPrincipalId,
    int nowUnixS,
  ) {
    if (!prompt.isActionableAt(nowUnixS)) {
      throw Spec006Error(
        ErrorCode.policy,
        'approval is not actionable (expired or not pending)',
        correlationId: prompt.correlation,
      );
    }
    if (!session.isUsableAt(nowUnixS)) {
      throw Spec006Error(
        ErrorCode.authorization,
        'session is not usable (revoked or expired)',
        correlationId: prompt.correlation,
      );
    }
    if (!binding.active) {
      throw Spec006Error(
        ErrorCode.authorization,
        'device binding is revoked',
        correlationId: prompt.correlation,
      );
    }
    if (actingDeviceId != prompt.deviceId) {
      throw Spec006Error(
        ErrorCode.authorization,
        'acting device does not match approval device binding',
        correlationId: prompt.correlation,
      );
    }
    if (actingPrincipalId != prompt.principalId) {
      throw Spec006Error(
        ErrorCode.authorization,
        'acting principal does not match approval principal binding',
        correlationId: prompt.correlation,
      );
    }
    final highRisk = prompt.risk == RiskClass.r3 || prompt.risk == RiskClass.r4;
    if (highRisk && !prompt.approvalClass.requiresHuman) {
      throw Spec006Error(
        ErrorCode.policy,
        'high-risk approval requires a human approval class',
        correlationId: prompt.correlation,
      );
    }
    // AUD-041: high-risk approvals require a cryptographic step-up
    // session. A single-factor (or merely multi-factor, non-step-up)
    // session is NEVER sufficient for R3/R4.
    if (highRisk && session.strength != SessionStrength.stepUp) {
      throw Spec006Error(
        ErrorCode.policy,
        'high-risk approval requires a STEP_UP session',
        correlationId: prompt.correlation,
      );
    }
    // AUD-041: the session must belong to the SAME principal, device,
    // and tenant named by the prompt. An approval cannot be resolved
    // with a session that does not own the acting identity.
    if (session.principalId != prompt.principalId ||
        session.deviceId != prompt.deviceId ||
        session.tenantId != prompt.tenantId) {
      throw Spec006Error(
        ErrorCode.authorization,
        'session identity does not match the approval binding',
        correlationId: prompt.correlation,
      );
    }
    // AUD-041: the device binding must actually OWN the device,
    // principal, and tenant named by the prompt. A binding for a
    // different device/user/tenant cannot authorize this approval.
    if (binding.deviceId != prompt.deviceId ||
        binding.principalId != prompt.principalId ||
        binding.tenantId != prompt.tenantId) {
      throw Spec006Error(
        ErrorCode.authorization,
        'device binding does not own the approval principal/device/tenant',
        correlationId: prompt.correlation,
      );
    }
  }

  /// Approves [prompt] after binding checks. Idempotent: a previously
  /// approved approval returns the same resolution; a previously
  /// denied approval is a CONFLICT.
  ApprovalResolution approve({
    required ApprovalPrompt prompt,
    required MobileSession session,
    required DeviceBinding binding,
    required String actingDeviceId,
    required String actingPrincipalId,
    required int nowUnixS,
  }) {
    final existing = _store.find(prompt.approvalId);
    if (existing != null) {
      if (existing.decision == ApprovalDecision.approved) {
        return existing;
      }
      throw Spec006Error(
        ErrorCode.conflict,
        'approval already resolved',
        correlationId: prompt.correlation,
      );
    }
    _guard(
      prompt,
      session,
      binding,
      actingDeviceId,
      actingPrincipalId,
      nowUnixS,
    );
    final resolution = ApprovalResolution(
      approvalId: prompt.approvalId,
      decision: ApprovalDecision.approved,
      decidedAtUnixS: nowUnixS,
      correlation: prompt.correlation,
    );
    _store.put(resolution);
    return resolution;
  }

  /// Denies [prompt] after binding checks. Idempotent: a previously
  /// denied approval returns the same resolution; a previously
  /// approved approval is a CONFLICT.
  ApprovalResolution deny({
    required ApprovalPrompt prompt,
    required MobileSession session,
    required DeviceBinding binding,
    required String actingDeviceId,
    required String actingPrincipalId,
    required int nowUnixS,
  }) {
    final existing = _store.find(prompt.approvalId);
    if (existing != null) {
      if (existing.decision == ApprovalDecision.denied) {
        return existing;
      }
      throw Spec006Error(
        ErrorCode.conflict,
        'approval already resolved',
        correlationId: prompt.correlation,
      );
    }
    _guard(
      prompt,
      session,
      binding,
      actingDeviceId,
      actingPrincipalId,
      nowUnixS,
    );
    final resolution = ApprovalResolution(
      approvalId: prompt.approvalId,
      decision: ApprovalDecision.denied,
      decidedAtUnixS: nowUnixS,
      correlation: prompt.correlation,
    );
    _store.put(resolution);
    return resolution;
  }
}
