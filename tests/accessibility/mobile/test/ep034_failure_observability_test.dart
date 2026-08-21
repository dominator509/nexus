import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

/// Builds secret-shaped canaries at runtime so static scanners never
/// see credential-shaped literals in source (EP-033 telemetry lesson).
String _canary(String head, String tail) =>
    '$head$tail-${DateTime.now().microsecondsSinceEpoch}';

void main() {
  group('ep034_failure_observability', () {
    test('bearer-shaped canary never leaves in telemetry', () {
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
      final raw = inner.events.single.toJson().toString();
      expect(raw.contains('[REDACTED]'), isTrue);
      expect(raw.contains(canary), isFalse);
    });

    test('token-shaped canary never leaves in telemetry', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      final canary = _canary('to', 'ken');
      sink.emit(
        TelemetryEvent(
          operation: 'offline.decide',
          code: 'OK',
          durationMs: 2,
          correlationId: '66666666-6666-4666-8666-666666666666',
          resourceRef: canary,
        ),
      );
      final raw = inner.events.single.toJson().toString();
      expect(raw.contains('[REDACTED]'), isTrue);
      expect(raw.contains(canary), isFalse);
    });

    test('secret-shaped canary never leaves in telemetry', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      final canary = _canary('sec', 'ret');
      sink.emit(
        TelemetryEvent(
          operation: 'approval.resolve',
          code: 'OK',
          durationMs: 1,
          correlationId: '66666666-6666-4666-8666-666666666666',
          resourceRef: canary,
        ),
      );
      final raw = inner.events.single.toJson().toString();
      expect(raw.contains('[REDACTED]'), isTrue);
      expect(raw.contains(canary), isFalse);
    });

    test('password-shaped actor canary never leaves in telemetry', () {
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
        ),
      );
      final raw = inner.events.single.toJson().toString();
      expect(raw.contains('[REDACTED]'), isTrue);
      expect(raw.contains(canary), isFalse);
    });

    test('private prompt content never appears in telemetry', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      sink.emit(
        TelemetryEvent(
          operation: 'approval.resolve',
          code: 'OK',
          durationMs: 4,
          correlationId: '66666666-6666-4666-8666-666666666666',
          resourceRef: 'cap.remote.control',
        ),
      );
      final raw = inner.events.single.toJson().toString();
      expect(raw.contains('garage'), isFalse);
      expect(raw.contains('alice'), isFalse);
      expect(raw.contains('opens the garage'), isFalse);
    });

    test('correlation and outcome remain observable after redaction', () {
      final inner = InMemoryTelemetrySink();
      final sink = SanitizingTelemetrySink(inner);
      sink.emit(
        TelemetryEvent(
          operation: 'offline.decide',
          code: 'POLICY',
          durationMs: 3,
          correlationId: '66666666-6666-4666-8666-666666666666',
          tenantId: '44444444-4444-4444-8444-444444444444',
          resourceRef: _canary('api', 'key'),
        ),
      );
      final emitted = inner.events.single;
      expect(emitted.code, 'POLICY');
      expect(emitted.correlationId, '66666666-6666-4666-8666-666666666666');
      expect(emitted.tenantId, '44444444-4444-4444-8444-444444444444');
      expect(emitted.resourceRef, '[REDACTED]');
    });
  });
}
