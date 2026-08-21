import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// EP-034 M2 dependency-direction guard.
///
/// The mobile core-behavior package must stay provider-neutral and
/// framework-neutral: it may import the nexus_mobile contract layer
/// (pure Dart contracts) and nothing else. Native providers and
/// Flutter widgets are later milestones.
void main() {
  group('ep034_unit_dependency_direction', () {
    const behaviorFiles = <String>[
      'lib/nexus_mobile_contracts.dart',
      'lib/src/behavior/approval_binding.dart',
      'lib/src/behavior/offline_policy.dart',
      'lib/src/behavior/telemetry.dart',
    ];

    const forbidden = <String>[
      'package:http/',
      'package:dio/',
      'package:firebase_',
      'package:shared_preferences',
      'package:flutter_secure_storage',
      'package:web3dart',
      'package:grpc',
      'package:temporal_',
    ];

    for (final file in behaviorFiles) {
      test('$file imports only the allowed surface', () {
        final source = File(file).readAsStringSync();
        for (final line in source.split('\n')) {
          final trimmed = line.trim();
          if (!trimmed.startsWith('import ') &&
              !trimmed.startsWith('export ')) {
            continue;
          }
          for (final bad in forbidden) {
            expect(
              trimmed.contains(bad),
              isFalse,
              reason: '$file must not import $bad (line: $trimmed)',
            );
          }
        }
      });
    }

    test('behavior sources never import Flutter widgets or material', () {
      for (final file in behaviorFiles) {
        final source = File(file).readAsStringSync();
        expect(
          source.contains('package:flutter/material.dart'),
          isFalse,
          reason: '$file must stay framework-neutral',
        );
        expect(
          source.contains('package:flutter/widgets.dart'),
          isFalse,
          reason: '$file must stay framework-neutral',
        );
      }
    });

    test('behavior sources import only the nexus_mobile contract barrel', () {
      for (final file in behaviorFiles) {
        final source = File(file).readAsStringSync();
        for (final line in source.split('\n')) {
          final trimmed = line.trim();
          if (!trimmed.startsWith('import ') &&
              !trimmed.startsWith('export ')) {
            continue;
          }
          if (trimmed.contains("package:nexus_mobile/")) {
            expect(
              trimmed.contains('package:nexus_mobile/nexus_mobile.dart'),
              isTrue,
              reason:
                  '$file must import only the public contract barrel, not app code',
            );
          }
        }
      }
    });

    test('behavior directory has no provider SDK files', () {
      final entries = Directory('lib/src/behavior').listSync();
      for (final entry in entries) {
        final name = entry.path.split('/').last.toLowerCase();
        expect(
          name.contains('firebase') ||
              name.contains('aws') ||
              name.contains('azure') ||
              name.contains('google'),
          isFalse,
          reason: 'unexpected provider-named file: ${entry.path}',
        );
      }
    });
  });
}
