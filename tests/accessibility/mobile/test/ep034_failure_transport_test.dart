import 'dart:async';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import 'ep034_failure_support.dart';

void main() {
  late ApprovalApiServer server;
  late HttpClient client;

  setUp(() async {
    server = ApprovalApiServer();
    await server.start();
    client = HttpClient();
  });

  tearDown(() async {
    client.close(force: true);
    await server.close();
  });

  group('ep034_failure_transport', () {
    test('transport unavailable fails closed with SocketException', () async {
      final closedServer = ApprovalApiServer();
      await closedServer.start();
      final closedPort = closedServer.port;
      await closedServer.close();
      await expectLater(() async {
        final request = await client.getUrl(
          Uri.parse('http://127.0.0.1:$closedPort/healthz'),
        );
        await request.close();
      }, throwsA(isA<SocketException>()));
    });

    test('slow server response exceeds client timeout', () async {
      final request = await client.getUrl(
        Uri.parse('http://127.0.0.1:${server.port}/slow?ms=3000'),
      );
      await expectLater(
        request.close().timeout(const Duration(milliseconds: 150)),
        throwsA(isA<TimeoutException>()),
      );
    });

    test(
      'client cancellation aborts the in-flight request server-side',
      () async {
        final before = server.abortedRequestCount;
        final request = await client.postUrl(
          Uri.parse(
            'http://127.0.0.1:${server.port}/approvals/11111111-1111-4111-8111-111111111111/resolve',
          ),
        );
        request.headers.contentType = ContentType.json;
        request.write('{"approval_id": "11111111-');
        await request.flush();
        request.abort();
        await Future<void>.delayed(const Duration(milliseconds: 300));
        expect(server.abortedRequestCount, greaterThan(before));
      },
    );

    test('unknown route fails closed with NOT_FOUND', () async {
      final request = await client.getUrl(
        Uri.parse('http://127.0.0.1:${server.port}/fabricated/route'),
      );
      final response = await request.close();
      expect(response.statusCode, 404);
      await response.drain<void>();
    });
  });
}
