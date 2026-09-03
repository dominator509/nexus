import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

import 'ep034_integration_support.dart';

ApprovalPrompt _prompt({
  String approvalId = '11111111-1111-4111-8111-111111111111',
  String deviceId = '22222222-2222-4222-8222-222222222222',
  String principalId = '33333333-3333-4333-8333-333333333333',
  RiskClass risk = RiskClass.r4,
  ApprovalClass approvalClass = ApprovalClass.human,
  int expiresAtUnixS = 1000000000,
}) {
  return ApprovalPrompt(
    approvalId: approvalId,
    tenantId: '44444444-4444-4444-8444-444444444444',
    principalId: principalId,
    deviceId: deviceId,
    actionId: '55555555-5555-4555-8555-555555555555',
    capabilityId: 'cap.remote.control',
    risk: risk,
    approvalClass: approvalClass,
    requester: 'alice',
    target: 'garage-door',
    externalEffects: 'opens the garage door',
    cost: 'none',
    reversible: true,
    expiresAtUnixS: expiresAtUnixS,
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
    // AUD-041: high-risk (R4) approvals REQUIRE a STEP_UP session.
    // This suite round-trips R4 prompts over the real transport, so
    // the fixture session must be step-up - a multiFactor session
    // would (correctly) be refused by the guard.
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

Future<Map<String, dynamic>> _getJson(HttpClient client, String url) async {
  final request = await client.getUrl(Uri.parse(url));
  final response = await request.close();
  final body = await utf8.decoder.bind(response).join();
  return jsonDecode(body) as Map<String, dynamic>;
}

Future<(int, Map<String, dynamic>)> _postJson(
  HttpClient client,
  String url,
  Map<String, dynamic> body, {
  String? idempotencyKey,
  String? correlation,
}) async {
  final request = await client.postUrl(Uri.parse(url));
  if (idempotencyKey != null) {
    request.headers.set('x-idempotency-key', idempotencyKey);
  }
  if (correlation != null) {
    request.headers.set('x-correlation-id', correlation);
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

  group('ep034_integration_transport', () {
    test(
      'readiness: server health endpoint reports ok over real HTTP',
      () async {
        final health = await _getJson(
          client,
          'http://127.0.0.1:${server.port}/healthz',
        );
        expect(health['status'], 'ok');
      },
    );

    test(
      'approval prompt round-trips canonical JSON over real transport',
      () async {
        final wire = await _getJson(
          client,
          'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111',
        );
        final prompt = ApprovalPrompt.fromJson(wire);
        expect(prompt.risk, RiskClass.r4);
        expect(prompt.approvalClass, ApprovalClass.human);
        expect(prompt.capabilityId, 'cap.remote.control');

        final service = ApprovalBindingService();
        final resolution = service.approve(
          prompt: prompt,
          session: _session(),
          binding: _binding(),
          actingDeviceId: '22222222-2222-4222-8222-222222222222',
          actingPrincipalId: '33333333-3333-4333-8333-333333333333',
          nowUnixS: 500,
        );

        final (status, body) = await _postJson(
          client,
          'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111/resolve',
          resolution.toJson(),
          correlation: '66666666-6666-4666-8666-666666666666',
        );
        expect(status, 200);
        expect(body['decision'], 'APPROVED');
        expect(body['correlation'], '66666666-6666-4666-8666-666666666666');
      },
    );

    test('idempotent retry over transport resolves exactly once', () async {
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
      const key = 'idem-0000000000000001';
      final (firstStatus, firstBody) = await _postJson(
        client,
        url,
        resolution.toJson(),
        idempotencyKey: key,
      );
      final (secondStatus, secondBody) = await _postJson(
        client,
        url,
        resolution.toJson(),
        idempotencyKey: key,
      );
      expect(firstStatus, 200);
      expect(secondStatus, 200);
      expect(secondBody['approval_id'], firstBody['approval_id']);
      expect(secondBody['decided_at_unix_s'], firstBody['decided_at_unix_s']);
      final resolutionAudits = server.audit.where(
        (e) => e.path.endsWith('/resolve'),
      );
      expect(resolutionAudits.length, 2);
    });

    test('divergent retry with same idempotency key is CONFLICT', () async {
      final prompt = _prompt();
      final service = ApprovalBindingService();
      final approved = service.approve(
        prompt: prompt,
        session: _session(),
        binding: _binding(),
        actingDeviceId: '22222222-2222-4222-8222-222222222222',
        actingPrincipalId: '33333333-3333-4333-8333-333333333333',
        nowUnixS: 500,
      );
      final url =
          'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111/resolve';
      const key = 'idem-0000000000000002';
      await _postJson(client, url, approved.toJson(), idempotencyKey: key);
      final denied = ApprovalResolution(
        approvalId: '11111111-1111-4111-8111-111111111111',
        decision: ApprovalDecision.denied,
        decidedAtUnixS: 600,
        correlation: '66666666-6666-4666-8666-666666666666',
      );
      final (status, _) = await _postJson(
        client,
        url,
        denied.toJson(),
        idempotencyKey: key,
      );
      expect(status, 409);
    });

    test(
      'slow server response exceeds client timeout (TIMEOUT at transport)',
      () async {
        await expectLater(
          _getJson(
            client,
            'http://127.0.0.1:${server.port}/slow?ms=2000',
          ).timeout(const Duration(milliseconds: 150)),
          throwsA(isA<TimeoutException>()),
        );
      },
    );

    test(
      'client cancellation reaches the server as an aborted request',
      () async {
        final before = server.abortedRequestCount;
        final request = await client.postUrl(
          Uri.parse(
            'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111/resolve',
          ),
        );
        request.headers.contentType = ContentType.json;
        // Write a partial body then abort the request mid-flight.
        request.write('{"approval_id": "11111111-');
        await request.flush();
        request.abort();
        await Future<void>.delayed(const Duration(milliseconds: 300));
        expect(server.abortedRequestCount, greaterThan(before));
      },
    );

    test(
      'typed SPEC-006 error crosses transport with correlation preserved',
      () async {
        final (status, body) = await _postJson(
          client,
          'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111/resolve',
          <String, dynamic>{
            'approval_id': '11111111-1111-4111-8111-111111111111',
            'decision': 'FABRICATED',
            'decided_at_unix_s': 500,
            'correlation': '66666666-6666-4666-8666-666666666666',
          },
          correlation: '66666666-6666-4666-8666-666666666666',
        );
        expect(status, 422);
        final problem = ProblemDetails.fromJson(body);
        expect(problem.code, ErrorCode.vocabulary);
        expect(problem.correlationId, '66666666-6666-4666-8666-666666666666');
      },
    );

    test('server audit records correlation across the boundary', () async {
      await _getJson(
        client,
        'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111',
      );
      expect(server.audit, isNotEmpty);
      expect(
        server.audit.last.path,
        contains('11111111-1111-4111-8111-111111111111'),
      );
    });

    test('cleanup: server close releases the port for rebinding', () async {
      final releasedPort = server.port;
      await server.close();
      final rebind = await ServerSocket.bind(
        InternetAddress.loopbackIPv4,
        releasedPort,
      );
      await rebind.close();
    });

    test('transport unavailable fails closed (connection refused)', () async {
      final closedServer = ApprovalApiServer();
      await closedServer.start();
      final closedPort = closedServer.port;
      await closedServer.close();
      await expectLater(
        _getJson(client, 'http://127.0.0.1:$closedPort/healthz'),
        throwsA(isA<SocketException>()),
      );
    });
  });
}
