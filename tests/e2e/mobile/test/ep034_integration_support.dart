/// EP-034 M3 integration fixture: a REAL dart:io HTTP server bound to
/// a loopback socket, serving canonical mobile contract JSON.
///
/// This is the transport side of the integration proof. The domain
/// logic under test is the production mobile contract/behavior layer
/// (nexus_mobile + nexus_mobile_contracts); this server is a
/// controlled fixture inside the e2e test zone (TESTING.md) that
/// behaves like the Nexus backend over real HTTP semantics: health,
/// approval fetch, idempotent resolution, slow responses, and audit
/// event emission.
library;

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:nexus_mobile/nexus_mobile.dart';
import 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart';

/// One audited request as observed by the server.
class AuditEvent {
  AuditEvent(this.method, this.path, this.status, this.correlation);

  final String method;
  final String path;
  final int status;
  final String? correlation;
}

/// Real loopback HTTP server for the EP-034 M3 integration suite.
class ApprovalApiServer {
  ApprovalApiServer({Map<String, ApprovalPrompt>? approvals})
    : _approvals = approvals ?? <String, ApprovalPrompt>{};

  final Map<String, ApprovalPrompt> _approvals;
  final List<AuditEvent> audit = <AuditEvent>[];
  final Map<String, ApprovalResolution> _resolutions =
      <String, ApprovalResolution>{};
  final Map<String, String> _idempotency = <String, String>{};
  final List<int> _abortedRequests = <int>[];

  late final HttpServer _server;
  int get port => _server.port;
  int get abortedRequestCount => _abortedRequests.length;

  int _abortCounter = 0;

  Future<void> start() async {
    _server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    unawaited(_serve());
  }

  Future<void> _serve() async {
    await for (final request in _server) {
      unawaited(_handle(request));
    }
  }

  Future<void> _handle(HttpRequest request) async {
    final path = request.uri.path;
    final method = request.method;
    final correlation = request.headers.value('x-correlation-id');

    Future<void> respond(
      int status,
      Map<String, dynamic> body, {
      bool recordAudit = true,
    }) async {
      if (recordAudit) {
        audit.add(AuditEvent(method, path, status, correlation));
      }
      request.response.statusCode = status;
      request.response.headers.contentType = ContentType.json;
      request.response.write(jsonEncode(body));
      await request.response.close();
    }

    if (path == '/healthz' && method == 'GET') {
      await respond(200, <String, dynamic>{'status': 'ok'});
      return;
    }

    if (path.startsWith('/slow')) {
      final ms = int.tryParse(request.uri.queryParameters['ms'] ?? '') ?? 0;
      await Future<void>.delayed(Duration(milliseconds: ms));
      await respond(200, <String, dynamic>{'status': 'ok'});
      return;
    }

    final approvalMatch = RegExp(r'^/approvals/([0-9a-f-]+)$').firstMatch(path);
    if (approvalMatch != null && method == 'GET') {
      final id = approvalMatch.group(1)!;
      final prompt = _approvals[id];
      if (prompt == null) {
        await respond(404, <String, dynamic>{'code': 'NOT_FOUND'});
        return;
      }
      await respond(200, prompt.toJson());
      return;
    }

    final resolveMatch = RegExp(
      r'^/approvals/([0-9a-f-]+)/resolve$',
    ).firstMatch(path);
    if (resolveMatch != null && method == 'POST') {
      final id = resolveMatch.group(1)!;
      final idemKey = request.headers.value('x-idempotency-key');
      String body;
      try {
        body = await utf8.decoder.bind(request).join();
      } on HttpException {
        // Client aborted before completing the request body.
        _abortedRequests.add(++_abortCounter);
        try {
          await request.response.close();
        } catch (_) {
          // Socket already closed; nothing to write.
        }
        return;
      } on IOException {
        _abortedRequests.add(++_abortCounter);
        try {
          await request.response.close();
        } catch (_) {
          // Socket already closed; nothing to write.
        }
        return;
      }
      final decoded = jsonDecode(body) as Map<String, dynamic>;
      if (decoded['decision'] != 'APPROVED' &&
          decoded['decision'] != 'DENIED') {
        await respond(422, <String, dynamic>{
          'code': 'VOCABULARY',
          'detail': 'unknown decision',
          'correlation_id': correlation,
        });
        return;
      }
      final resolution = ApprovalResolution(
        approvalId: decoded['approval_id'] as String,
        decision: decoded['decision'] == 'APPROVED'
            ? ApprovalDecision.approved
            : ApprovalDecision.denied,
        decidedAtUnixS: decoded['decided_at_unix_s'] as int,
        correlation: decoded['correlation'] as String,
      );
      if (idemKey != null && _idempotency.containsKey(idemKey)) {
        final prior = _resolutions[_idempotency[idemKey]];
        if (prior != null &&
            prior.decision == resolution.decision &&
            prior.decidedAtUnixS == resolution.decidedAtUnixS &&
            prior.correlation == resolution.correlation) {
          await respond(200, prior.toJson());
          return;
        }
        await respond(409, <String, dynamic>{'code': 'CONFLICT'});
        return;
      }
      _resolutions[id] = resolution;
      if (idemKey != null) {
        _idempotency[idemKey] = id;
      }
      await respond(200, resolution.toJson());
      return;
    }

    await respond(404, <String, dynamic>{'code': 'NOT_FOUND'});
  }

  Future<void> close() async {
    await _server.close(force: true);
  }
}
