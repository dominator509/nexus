import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// EP-034 M1 dependency-direction guard.
///
/// The mobile contract layer must stay provider-neutral and never
/// import backend clients, web framework code, or third-party
/// provider SDKs. This test scans the contract sources and fails on
/// any forbidden import.
void main() {
  group('ep034_unit_dependency_direction', () {
    const contractFiles = <String>[
      'lib/nexus_mobile.dart',
      'lib/src/contracts/errors.dart',
      'lib/src/contracts/validate.dart',
      'lib/src/contracts/device.dart',
      'lib/src/contracts/session.dart',
      'lib/src/contracts/approvals.dart',
      'lib/src/contracts/enrollment.dart',
      'lib/src/contracts/voice.dart',
      'lib/src/contracts/bluetooth.dart',
      'lib/src/contracts/secure_store.dart',
      'lib/src/contracts/push.dart',
      'lib/src/contracts/remote.dart',
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

    for (final file in contractFiles) {
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

    test('contract sources never import Flutter widgets or material', () {
      for (final file in contractFiles) {
        if (file.startsWith('lib/src/contracts/')) {
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
      }
    });

    test('contracts directory has no provider SDK files', () {
      final entries = Directory('lib/src/contracts').listSync();
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
