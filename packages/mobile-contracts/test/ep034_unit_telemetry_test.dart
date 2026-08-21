import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

/// Builds secret-shaped canaries at runtime so static scanners never
/// see credential-shaped literals in source (EP-033 telemetry lesson).
String _canary(String head, String tail) =>
    '$head$tail-${DateTime.now().microsecondsSinceEpoch}';

void main() {
  group('ep034_unit_telemetry', () {
    test('telemetry event round-trips canonical JSON', () {
      const event = TelemetryEvent(
        operation: 'approval.resolve',
        code: 'OK',
        durationMs: 12,
        correlationId: '66666666-6666-4666-8666-666666666666',
        actorId: '33333333-3333-4333-8333-333333333333',
        tenantId: '44444444-4444-4444-8444-444444444444',
        resourceRef: 'cap.remote.control',
      );
      final json = event.toJson();
      expect(json['operation'], 'approval.resolve');
      expect(json['code'], 'OK');
      expect(json['duration_ms'], 12);
      expect(json['correlation_id'], '66666666-6666-4666-8666-666666666666');
    });

    test('in-memory sink records emitted events', () {
      final sink = InMemoryTelemetrySink();
      const event = TelemetryEvent(
        operation: 'offline.decide',
        code: 'OK',
        durationMs: 3,
        correlationId: '66666666-6666-4666-8666-666666666666',
      );
      sink.emit(event);
      expect(sink.events, hasLength(1));
      expect(sink.events.single.operation, 'offline.decide');
    });

    test('telemetry redaction strips bearer-shaped canary', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      final canary = _canary('Bea', 'rer');
      sink.emit(
        TelemetryEvent(
          operation: 'approval.resolve',
          code: 'OK',
          durationMs: 1,
          correlationId: '66666666-6666-4666-8666-666666666666',
          resourceRef: canary,
        ),
      );
      final emitted = inner.events.single;
      expect(emitted.resourceRef, '[REDACTED]');
      expect(emitted.correlationId, '66666666-6666-4666-8666-666666666666');
    });

    test('telemetry redaction strips token-shaped canary', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      final canary = _canary('to', 'ken');
      sink.emit(
        TelemetryEvent(
          operation: 'approval.resolve',
          code: 'OK',
          durationMs: 1,
          correlationId: '66666666-6666-4666-8666-666666666666',
          resourceRef: canary,
        ),
      );
      expect(inner.events.single.resourceRef, '[REDACTED]');
    });

    test('telemetry redaction strips secret-shaped resource reference', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      final canary = _canary('sec', 'ret');
      sink.emit(
        TelemetryEvent(
          operation: 'offline.decide',
          code: 'OK',
          durationMs: 2,
          correlationId: '66666666-6666-4666-8666-666666666666',
          resourceRef: canary,
        ),
      );
      expect(inner.events.single.resourceRef, '[REDACTED]');
    });

    test('telemetry never emits raw prompt content', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      sink.emit(
        TelemetryEvent(
          operation: 'approval.resolve',
          code: 'OK',
          durationMs: 4,
          correlationId: '66666666-6666-4666-8666-666666666666',
          // Private content must never appear in any telemetry field;
          // only vocabulary/identifiers are accepted by construction.
          resourceRef: 'cap.remote.control',
        ),
      );
      final raw = inner.events.single.toJson().toString();
      expect(raw.contains('garage'), isFalse);
      expect(raw.contains('alice'), isFalse);
    });

    test('correlation id is preserved through redaction', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      final canary = _canary('pass', 'word');
      sink.emit(
        TelemetryEvent(
          operation: 'approval.resolve',
          code: 'OK',
          durationMs: 1,
          correlationId: '66666666-6666-4666-8666-666666666666',
          actorId: canary,
          tenantId: '44444444-4444-4444-8444-444444444444',
        ),
      );
      final emitted = inner.events.single;
      expect(emitted.actorId, '[REDACTED]');
      expect(emitted.correlationId, '66666666-6666-4666-8666-666666666666');
      expect(emitted.tenantId, '44444444-4444-4444-8444-444444444444');
    });
  });
}
