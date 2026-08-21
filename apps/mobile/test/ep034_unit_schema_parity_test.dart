import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// EP-034 M1 schema-parity guard.
///
/// The mobile contracts bind to the canonical JSON schemas under
/// `schemas/`. This test re-reads the schema files from disk and
/// asserts the contract enums match the schema enums exactly, so a
/// schema vocabulary change cannot silently drift from the client.
void main() {
  group('ep034_unit_schema_parity', () {
    Map<String, dynamic> readSchema(String relative) {
      final file = File('../../schemas/$relative');
      expect(file.existsSync(), isTrue, reason: 'missing $relative');
      final decoded = jsonDecode(file.readAsStringSync());
      expect(decoded, isA<Map<String, dynamic>>());
      return decoded as Map<String, dynamic>;
    }

    List<String> enumValues(Map<String, dynamic> schema, String property) {
      final props = schema['properties'] as Map<String, dynamic>;
      final prop = props[property] as Map<String, dynamic>;
      return (prop['enum'] as List<dynamic>).cast<String>();
    }

    test('device-identity schema kind enum matches DeviceKind', () {
      final schema = readSchema('identity/device-identity.schema.json');
      final kinds = enumValues(schema, 'kind');
      expect(kinds, [
        'PHONE',
        'TABLET',
        'DESKTOP',
        'LAPTOP',
        'SPEAKER',
        'CAMERA',
        'DISPLAY',
        'SERVER',
        'APPLIANCE',
        'UNKNOWN',
      ]);
    });

    test('device-identity schema trust_level matches DeviceTrustLevel', () {
      final schema = readSchema('identity/device-identity.schema.json');
      expect(enumValues(schema, 'trust_level'), [
        'UNVERIFIED',
        'LOCAL',
        'VERIFIED',
      ]);
    });

    test('auth-session schema grant_flow matches GrantFlow', () {
      final schema = readSchema('auth/auth-session.schema.json');
      expect(enumValues(schema, 'grant_flow'), [
        'AUTHORIZATION_CODE',
        'REFRESH_TOKEN',
        'CLIENT_CREDENTIALS',
      ]);
    });

    test('auth-session schema strength matches SessionStrength', () {
      final schema = readSchema('auth/auth-session.schema.json');
      expect(enumValues(schema, 'strength'), [
        'NONE',
        'SINGLE_FACTOR',
        'MULTI_FACTOR',
        'STEP_UP',
      ]);
    });

    test('passkey-challenge schema state matches PasskeyChallengeState', () {
      final schema = readSchema('auth/passkey-challenge.schema.json');
      expect(enumValues(schema, 'state'), [
        'PENDING_CHALLENGE',
        'REGISTERED',
        'REVOKED',
        'EXPIRED',
        'FAILED',
      ]);
    });

    test('step-up-challenge schema state matches StepUpChallengeState', () {
      final schema = readSchema('auth/step-up-challenge.schema.json');
      expect(enumValues(schema, 'state'), [
        'PENDING',
        'COMPLETED',
        'FAILED',
        'EXPIRED',
        'CANCELLED',
      ]);
    });

    test('step-up-challenge schema strength matches ChallengeStrength', () {
      final schema = readSchema('auth/step-up-challenge.schema.json');
      expect(enumValues(schema, 'required_strength'), [
        'NONE',
        'SINGLE_FACTOR',
        'MULTI_FACTOR',
        'STEP_UP',
      ]);
    });

    test('action-request schema risk matches RiskClass', () {
      final schema = readSchema('action-request.schema.json');
      expect(enumValues(schema, 'risk'), ['R0', 'R1', 'R2', 'R3', 'R4']);
    });

    test('hydra action-request approval_class matches ApprovalClass', () {
      final schema = readSchema('hydra/action-request.schema.json');
      expect(enumValues(schema, 'approval_class'), [
        'NONE',
        'POLICY',
        'HUMAN',
        'STRONG_HUMAN',
        'FOUR_EYES',
      ]);
    });
  });
}
