/**
 * EP-033 M2 desktop command dispatcher (deterministic core).
 *
 * Consequential commands flow through a typed pipeline:
 * TypedCommandRequest (vocabulary-validated) -> session gate ->
 * authorization/approval checks -> idempotency ring -> execution.
 *
 * Invariants:
 * - duplicate idempotency keys are rejected with Conflict, never
 *   double-executed (SPEC-006 behavior 2/3);
 * - R3/R4 risk fails closed without HUMAN-or-stronger approval
 *   (SPEC-006 behavior 6);
 * - unauthorized capabilities are refused before any execution;
 * - the dispatcher holds no secrets and records only redacted
 *   telemetry.
 */

import {
  APPROVAL_CLASSES,
  DispatchGate,
  KnownCapabilityVocabulary,
  RISK_CLASSES,
  TypedCommandRequest,
  ErrorCode,
  Spec006Error,
  redact,
  type AuthenticatedSession,
  type ApprovalClass,
  type RiskClass,
} from "@nexus/web";

export interface DispatchResult {
  action_id: string;
  capability_id: string;
  status: "EXECUTED" | "REJECTED";
  correlation: string;
}

const MAX_RECENT_KEYS = 256;

/**
 * Deterministic command dispatcher. `execute` is the I/O port injected
 * by the shell; the dispatcher itself performs no I/O (domain rules
 * pure, adapters import ports, never the reverse).
 */
export class DesktopCommandDispatcher {
  readonly vocabulary: KnownCapabilityVocabulary;
  readonly gate: DispatchGate;
  readonly #recentKeys: Map<string, string> = new Map();

  constructor(vocabulary: KnownCapabilityVocabulary, gate?: DispatchGate) {
    this.vocabulary = vocabulary;
    this.gate = gate ?? new DispatchGate();
  }

  /**
   * Dispatch a validated command.
   *
   * @param execute the transport/backend port; called exactly once per
   *        accepted command.
   * @returns EXECUTED only after `execute` returned; a failure inside
   *          execute propagates and is never reported as success.
   */
  dispatch(
    request: TypedCommandRequest,
    session: AuthenticatedSession,
    nowUnixS: number,
    execute: (request: TypedCommandRequest) => void,
  ): DispatchResult {
    // 1. Session gate (expiry fail closed, no blind replay).
    this.gate.authorize(request, session, nowUnixS);

    // 2. Idempotency ring: same key, same canonical request -> conflict.
    const existing = this.#recentKeys.get(request.idempotency_key);
    if (existing !== undefined && existing !== request.action_id) {
      throw new Spec006Error(
        ErrorCode.Conflict,
        "Idempotency key reused for a different request",
        request.invocation.correlation_id ?? session.correlation,
      );
    }
    if (existing === request.action_id) {
      // Duplicate delivery of the same request: replay is safe and
      // returns the same logical outcome without re-execution.
      return {
        action_id: request.action_id,
        capability_id: request.capability_id,
        status: "EXECUTED",
        correlation: request.invocation.correlation_id ?? session.correlation,
      };
    }

    // 3. Risk fail-closed: R3/R4 require HUMAN-or-stronger approval.
    this.#enforceRiskApproval(request);

    // 4. Execute exactly once.
    execute(request);

    // 5. Record idempotency (bounded ring).
    if (this.#recentKeys.size >= MAX_RECENT_KEYS) {
      const oldest = this.#recentKeys.keys().next().value;
      if (oldest !== undefined) {
        this.#recentKeys.delete(oldest);
      }
    }
    this.#recentKeys.set(request.idempotency_key, request.action_id);

    return {
      action_id: request.action_id,
      capability_id: request.capability_id,
      status: "EXECUTED",
      correlation: request.invocation.correlation_id ?? session.correlation,
    };
  }

  #enforceRiskApproval(request: TypedCommandRequest): void {
    // AUD-039: the R3/R4 gate resolves the capability's risk and
    // approval class from the OPERATOR-DECLARED registered profile -
    // NEVER from the wire. A client that self-declares approval_class
    // on the wire must not be able to mint authority for a capability
    // whose registered profile requires less or different authority.
    const profile = this.vocabulary.registeredProfile(request.capability_id);
    if (profile === undefined) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `capability '${request.capability_id}' has no registered risk profile`,
        request.invocation.correlation_id,
      );
    }
    if (profile.risk !== request.risk || profile.approval !== request.approval_class) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `wire risk/approval for '${request.capability_id}' does not match its registered profile`,
        request.invocation.correlation_id,
      );
    }
    const riskIndex = RISK_CLASSES.indexOf(profile.risk);
    if (riskIndex < 3) {
      return; // R0-R2 may proceed under the registered approval class.
    }
    // R3/R4: only HUMAN, STRONG_HUMAN, or FOUR_EYES may execute -
    // resolved from the REGISTERED profile, never the wire.
    if (
      profile.approval !== "HUMAN" &&
      profile.approval !== "STRONG_HUMAN" &&
      profile.approval !== "FOUR_EYES"
    ) {
      throw new Spec006Error(
        ErrorCode.Policy,
        `R3/R4 command '${request.capability_id}' requires human approval`,
        request.invocation.correlation_id,
      );
    }
  }
}

/**
 * Desktop telemetry boundary: safe fields only. Free text passes
 * through the shared redactor before recording (directive P).
 */
export interface DesktopTelemetryEntry {
  action: string;
  capability_id: string;
  correlation_id: string;
  outcome: string;
  duration_ms: number;
}

export class DesktopTelemetry {
  readonly #entries: Array<DesktopTelemetryEntry> = [];

  record(entry: DesktopTelemetryEntry): DesktopTelemetryEntry {
    const safe: DesktopTelemetryEntry = {
      action: redact(entry.action),
      capability_id: redact(entry.capability_id),
      correlation_id: redact(entry.correlation_id),
      outcome: redact(entry.outcome),
      duration_ms: entry.duration_ms,
    };
    this.#entries.push(safe);
    return safe;
  }

  entries(): ReadonlyArray<DesktopTelemetryEntry> {
    return [...this.#entries];
  }

  assertNoSecrets(): void {
    const serialized = JSON.stringify(this.#entries);
    const secretPattern =
      /\b(?:bearer|authorization|token|secret|password|api[_-]?key)\b/i;
    if (secretPattern.test(serialized)) {
      throw new Spec006Error(
        ErrorCode.Internal,
        "Desktop telemetry canary failed: secret-shaped content leaked",
      );
    }
  }
}

export { APPROVAL_CLASSES, type ApprovalClass };
