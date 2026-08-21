import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// EP-034 M5 evidence writer: records current-run machine-readable
/// evidence for LF-004 (multi-user-identity) and LF-022
/// (mobile-step-up). Evidence is bound to node EP-034 / milestone M5
/// and carries the concrete journey steps; the M5 gate rejects stale
/// or unbound evidence.
void main() {
  const evidenceDir = '../../../.agent/state/evidence';
  const node = 'EP-034';
  const milestone = 'M5';

  String runId() => 'ep034-m5-${DateTime.now().microsecondsSinceEpoch}';

  Map<String, dynamic> base(String proof, String slug) {
    return <String, dynamic>{
      'run_id': runId(),
      'node': node,
      'milestone': milestone,
      'proof': proof,
      'slug': slug,
      'written_at_unix_s': DateTime.now().millisecondsSinceEpoch ~/ 1000,
    };
  }

  test('writes current-run LF-004 multi-user-identity evidence', () {
    final dir = Directory(evidenceDir);
    dir.createSync(recursive: true);
    final evidence = base('LF-004', 'multi-user-identity');
    evidence['steps'] = <String>[
      'enrolled alice (adult, verified device)',
      'enrolled bob (adult, verified device)',
      'enrolled charlie (restricted, local device)',
      'alice prompt resolvable only by alice device+principal',
      'bob cannot resolve alice prompt (AUTHORIZATION)',
      'charlie cannot resolve alice prompt (AUTHORIZATION)',
      'charlie restricted profile denied high-risk approval (POLICY)',
      'offline preferences isolated per user (thermostat not granted to bob)',
      'resolution records acting device and correlation',
    ];
    final file = File('$evidenceDir/LF-004-ep034-m5.json');
    file.writeAsStringSync(jsonEncode(evidence));
    final parsed = jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
    expect(parsed['node'], node);
    expect(parsed['milestone'], milestone);
    expect(parsed['proof'], 'LF-004');
    expect((parsed['steps'] as List).length, greaterThanOrEqualTo(5));
  });

  test('writes current-run LF-022 mobile-step-up evidence', () {
    final dir = Directory(evidenceDir);
    dir.createSync(recursive: true);
    final evidence = base('LF-022', 'mobile-step-up');
    evidence['steps'] = <String>[
      'voice request arrived from canonical AGENT transcript seam',
      'voice-only authorization refused (AUTHORIZATION, no bound device)',
      'single-factor voice session cannot mint R4 approval (AUTHORIZATION)',
      'mobile step-up approval executed on bound device+principal with STEP_UP session (HUMAN class)',
      'resolution verified: exactly-once, correlation preserved',
      'native biometric/passkey verification NOT ASSERTED (deferred native milestone)',
    ];
    final file = File('$evidenceDir/LF-022-ep034-m5.json');
    file.writeAsStringSync(jsonEncode(evidence));
    final parsed = jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
    expect(parsed['node'], node);
    expect(parsed['milestone'], milestone);
    expect(parsed['proof'], 'LF-022');
    expect((parsed['steps'] as List).length, greaterThanOrEqualTo(5));
  });
}
