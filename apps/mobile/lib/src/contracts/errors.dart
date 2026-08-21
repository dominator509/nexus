/// EP-034 M1 canonical error vocabulary (SPEC-006).
///
/// Every mobile-visible failure uses a stable machine code and safe
/// human explanation. The client never collapses distinct failure
/// classes into a generic "Something went wrong": presentation may be
/// friendly, state must remain truthful.
///
/// The set mirrors the repository canonical vocabulary used by the web
/// contract layer (apps/web/src/contracts/errors.ts) and the Rust
/// notification crate: validation, authentication, authorization,
/// policy, not found, conflict, unavailable, timeout, rate limit,
/// external provider, verification, compensation, internal invariant,
/// plus the vocabulary rejection code.
library;

/// Canonical SPEC-006 error codes used by the mobile contracts.
enum ErrorCode {
  validation('VALIDATION'),
  authentication('AUTHENTICATION'),
  authorization('AUTHORIZATION'),
  policy('POLICY'),
  notFound('NOT_FOUND'),
  conflict('CONFLICT'),
  unavailable('UNAVAILABLE'),
  timeout('TIMEOUT'),
  rateLimit('RATE_LIMIT'),
  external('EXTERNAL'),
  verification('VERIFICATION'),
  compensation('COMPENSATION'),
  internal('INTERNAL'),
  vocabulary('VOCABULARY');

  const ErrorCode(this.wire);

  /// Stable machine code, e.g. "POLICY". Never free-form prose.
  final String wire;
}

/// RFC 9457-compatible problem details for mobile/HTTP boundaries.
class ProblemDetails {
  const ProblemDetails({
    required this.code,
    required this.type,
    required this.detail,
    this.correlationId,
    required this.status,
  });

  factory ProblemDetails.fromJson(Map<String, dynamic> json) {
    final codeWire = json['code'];
    final code = ErrorCode.values.where((c) => c.wire == codeWire).firstOrNull;
    if (code == null) {
      throw Spec006Error(ErrorCode.vocabulary, 'unknown error code: $codeWire');
    }
    final status = json['status'];
    return ProblemDetails(
      code: code,
      type: json['type'] as String? ?? '',
      detail: json['detail'] as String? ?? '',
      correlationId: json['correlation_id'] as String?,
      status: status is int ? status : 0,
    );
  }

  final ErrorCode code;

  /// RFC 9457-compatible problem type URI fragment.
  final String type;

  /// Safe human explanation. Never contains secrets or private content.
  final String detail;

  /// Canonical correlation id when available.
  final String? correlationId;

  /// HTTP status for the class when rendered over HTTP.
  final int status;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'code': code.wire,
    'type': type,
    'detail': detail,
    if (correlationId != null) 'correlation_id': correlationId,
    'status': status,
  };
}

int _statusFor(ErrorCode code) => switch (code) {
  ErrorCode.validation => 400,
  ErrorCode.authentication => 401,
  ErrorCode.authorization => 403,
  ErrorCode.policy => 403,
  ErrorCode.notFound => 404,
  ErrorCode.conflict => 409,
  ErrorCode.unavailable => 503,
  ErrorCode.timeout => 504,
  ErrorCode.rateLimit => 429,
  ErrorCode.external => 502,
  ErrorCode.verification => 409,
  ErrorCode.compensation => 500,
  ErrorCode.internal => 500,
  ErrorCode.vocabulary => 422,
};

/// Typed failure carrying the canonical code and optional correlation.
class Spec006Error implements Exception {
  Spec006Error(this.code, this.detail, {this.correlationId});

  final ErrorCode code;
  final String detail;
  final String? correlationId;

  ProblemDetails toProblemDetails() => ProblemDetails(
    code: code,
    type: 'https://schemas.nexus.local/problems/${code.wire.toLowerCase()}',
    detail: detail,
    correlationId: correlationId,
    status: _statusFor(code),
  );

  @override
  String toString() =>
      'Spec006Error(${code.wire}: $detail${correlationId == null ? '' : ' [$correlationId]'})';
}
