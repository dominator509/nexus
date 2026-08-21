/// EP-034 M2 canonical telemetry behavior (ExecPlan M2 content 5:
/// instrument public operations with the canonical telemetry context
/// but never emit secrets, prompts, raw audio, raw video, or private
/// content).
///
/// Events carry operation identity, outcome code, duration, and
/// canonical correlation. The sanitizing sink strips secret-shaped
/// values before emission (mirrors the canonical redaction contract
/// from the desktop telemetry path); a secret-shaped value can never
/// leave the device in telemetry.
library;

import 'package:nexus_mobile/nexus_mobile.dart';

/// Sanitized telemetry event. Fields are vocabulary or identifiers;
/// never free-form private content.
class TelemetryEvent {
  const TelemetryEvent({
    required this.operation,
    required this.code,
    required this.durationMs,
    required this.correlationId,
    this.actorId,
    this.tenantId,
    this.resourceRef,
  });

  final String operation;
  final String code;
  final int durationMs;
  final String correlationId;
  final String? actorId;
  final String? tenantId;
  final String? resourceRef;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'operation': operation,
    'code': code,
    'duration_ms': durationMs,
    'correlation_id': correlationId,
    if (actorId != null) 'actor_id': actorId,
    if (tenantId != null) 'tenant_id': tenantId,
    if (resourceRef != null) 'resource_ref': resourceRef,
  };
}

/// Port for telemetry emission.
abstract class TelemetrySink {
  void emit(TelemetryEvent event);
}

/// In-memory telemetry sink (M2 implementation).
class InMemoryTelemetrySink implements TelemetrySink {
  final List<TelemetryEvent> events = <TelemetryEvent>[];

  @override
  void emit(TelemetryEvent event) => events.add(event);
}

/// Redaction guard: replaces secret-shaped values with [REDACTED]
/// before forwarding. Secret shapes cover bearer tokens, JWT/opaque
/// tokens, api keys, passwords, and authorization material.
class SanitizingTelemetrySink implements TelemetrySink {
  SanitizingTelemetrySink(this.inner);

  static const _redacted = '[REDACTED]';
  static final _secretShape = RegExp(
    r'(bearer|jwt|token|secret|password|api[_-]?key|authorization)',
    caseSensitive: false,
  );

  final TelemetrySink inner;

  String _sanitize(String? value) {
    if (value == null) return '';
    return _secretShape.hasMatch(value) ? _redacted : value;
  }

  @override
  void emit(TelemetryEvent event) {
    inner.emit(
      TelemetryEvent(
        operation: _sanitize(event.operation),
        code: _sanitize(event.code),
        durationMs: event.durationMs,
        correlationId: _sanitize(event.correlationId),
        actorId: _sanitize(event.actorId),
        tenantId: _sanitize(event.tenantId),
        resourceRef: _sanitize(event.resourceRef),
      ),
    );
  }
}

/// Convenience error for telemetry misuse; internal invariant.
class TelemetryError extends Spec006Error {
  TelemetryError(String detail, {String? correlationId})
    : super(ErrorCode.internal, detail, correlationId: correlationId);
}
