import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

import 'ep034_failure_support.dart';

ApprovalPrompt _prompt() {
  return ApprovalPrompt(
    approvalId: '11111111-1111-4111-8111-111111111111',
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: '33333333-3333-4333-8333-333333333333',
    deviceId: '22222222-2222-4222-8222-222222222222',
    actionId: '55555555-5555-4555-8555-555555555555',
    capabilityId: 'cap.remote.control',
    risk: RiskClass.r4,
    approvalClass: ApprovalClass.human,
    requester: 'alice',
    target: 'garage-door',
    externalEffects: 'opens the garage door',
    cost: 'none',
    reversible: true,
    expiresAtUnixS: 1000000000,
    state: ApprovalPromptState.pending,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

MobileSession _session() {
  return MobileSession(
    sessionId: '77777777-7777-4777-8777-777777777777',
    principalId: '33333333-3333-4333-8333-333333333333',
    tenantId: '44444444-4444-4444-8444-444444444444',
    deviceId: '22222222-2222-4222-8222-222222222222',
    grantFlow: GrantFlow.authorizationCode,
    // AUD-041: R4 approvals require a STEP_UP session. These tests
    // exercise idempotency semantics, not session-strength policy
    // (the denial path has its own authority tests), so the fixture
    // session must satisfy the guard.
    strength: SessionStrength.stepUp,
    createdAtUnixS: 0,
    expiresAtUnixS: 1000000000,
    revoked: false,
    correlation: '66666666-6666-4666-8666-666666666666',
  );
}

DeviceBinding _binding() {
  return DeviceBinding(
    deviceId: '22222222-2222-4222-8222-222222222222',
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: '33333333-3333-4333-8333-333333333333',
    boundAtUnixS: 0,
    revoked: false,
  );
}

Future<(int, Map<String, dynamic>)> _post(
  HttpClient client,
  String url,
  Map<String, dynamic> body, {
  String? idempotencyKey,
}) async {
  final request = await client.postUrl(Uri.parse(url));
  if (idempotencyKey != null) {
    request.headers.set('x-idempotency-key', idempotencyKey);
  }
  request.headers.contentType = ContentType.json;
  request.write(jsonEncode(body));
  final response = await request.close();
  final responseBody = await utf8.decoder.bind(response).join();
  return (
    response.statusCode,
    responseBody.isEmpty
        ? <String, dynamic>{}
        : jsonDecode(responseBody) as Map<String, dynamic>,
  );
}

void main() {
  late ApprovalApiServer server;
  late HttpClient client;

  setUp(() async {
    server = ApprovalApiServer(
      approvals: <String, ApprovalPrompt>{
        '11111111-1111-4111-8111-111111111111': _prompt(),
      },
    );
    await server.start();
    client = HttpClient();
  });

  tearDown(() async {
    client.close(force: true);
    await server.close();
  });

  group('ep034_failure_idempotency', () {
    test('duplicate approval resolution executes exactly once', () {
      final service = ApprovalBindingService();
      final first = service.approve(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      final second = service.approve(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 600,
      );
      expect(identical(first, second), isTrue);
      expect(first.decidedAtUnixS, 500);
    });

    test('divergent re-resolution of a resolved approval is CONFLICT', () {
      final service = ApprovalBindingService();
      service.approve(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      expect(
        () => service.deny(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 600,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.conflict),
        ),
      );
    });

    test('double-deny is idempotent; approve after deny is CONFLICT', () {
      final service = ApprovalBindingService();
      final first = service.deny(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      final second = service.deny(
        prompt: _prompt(),
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 600,
      );
      expect(identical(first, second), isTrue);
      expect(
        () => service.approve(
          prompt: _prompt(),
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 700,
        ),
        throwsA(
          isA<Spec006Error>().having((e) => e.code, 'code', ErrorCode.conflict),
        ),
      );
    });

    test(
      'partial side effect: timed-out resolve retried with same key does not double-execute',
      () async {
        final prompt = _prompt();
        final service = ApprovalBindingService();
        final resolution = service.approve(
          prompt: prompt,
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        );
        final url =
            'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111/resolve';
        const key = 'idem-partial-0000000001';
        // First attempt lands on the server.
        final (firstStatus, firstBody) = await _post(
          client,
          url,
          resolution.toJson(),
          idempotencyKey: key,
        );
        expect(firstStatus, 200);
        // Client-side timeout appears to fail, but the retry with the same
        // key must replay the same resolution, not execute again.
        final (secondStatus, secondBody) = await _post(
          client,
          url,
          resolution.toJson(),
          idempotencyKey: key,
        );
        expect(secondStatus, 200);
        expect(secondBody['decided_at_unix_s'], firstBody['decided_at_unix_s']);
        expect(secondBody['approval_id'], firstBody['approval_id']);
      },
    );

    test(
      'corrupted resolution payload over transport is 422 VOCABULARY',
      () async {
        final (status, body) = await _post(
          client,
          'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111/resolve',
          <String, dynamic>{
            'approval_id': '11111111-1111-4111-8111-111111111111',
            'decision': 'MAYBE',
            'decided_at_unix_s': 500,
            'correlation': '66666666-6666-4666-8666-666666666666',
          },
        );
        expect(status, 422);
        expect(body['code'], 'VOCABULARY');
      },
    );
  });
}
