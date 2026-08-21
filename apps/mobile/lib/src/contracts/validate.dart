/// EP-034 M1 deny-unknown validation helpers.
///
/// Every wire/transport input entering the mobile contract layer is
/// validated with deny-unknown semantics, mirroring the canonical
/// schema `additionalProperties: false` and the serde
/// deny_unknown_fields pattern used by the Rust crates and the web
/// contract layer. Raw input can never fabricate vocabulary or
/// authority.
library;

import 'errors.dart';

/// Throws [Spec006Error] (VOCABULARY) if [json] carries a key outside
/// [allowedKeys]. This is the deny-unknown gate. Callers run this
/// ONCE at the top of every fromJson; the typed readers below then
/// read known keys without re-validating the whole map.
void rejectUnknownKeys(Map<String, dynamic> json, Set<String> allowedKeys) {
  for (final key in json.keys) {
    if (!allowedKeys.contains(key)) {
      throw Spec006Error(ErrorCode.vocabulary, 'unknown field: $key');
    }
  }
}

/// Reads a required [String] at [key]; throws VALIDATION if absent or
/// non-string. Assumes the caller already ran [rejectUnknownKeys].
String requireString(Map<String, dynamic> json, String key) {
  final value = json[key];
  if (value is! String) {
    throw Spec006Error(ErrorCode.validation, '$key must be a string');
  }
  return value;
}

/// Reads a required canonical UUID (8-4-4-4-12 hex) at [key].
String requireUuid(Map<String, dynamic> json, String key) {
  final value = requireString(json, key);
  if (!_uuidPattern.hasMatch(value)) {
    throw Spec006Error(ErrorCode.validation, '$key must be a uuid');
  }
  return value;
}

final RegExp _uuidPattern = RegExp(
  r'^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
);

/// Reads a canonical enum from a wire string at [key]; unknown values
/// are rejected with VOCABULARY, never defaulted.
T requireEnum<T extends Enum>(
  Map<String, dynamic> json,
  String key,
  List<T> values, {
  required String Function(T) wireOf,
}) {
  final raw = requireString(json, key);
  for (final value in values) {
    if (wireOf(value) == raw) {
      return value;
    }
  }
  throw Spec006Error(ErrorCode.vocabulary, 'unknown $key value: $raw');
}

/// Reads an idempotency key per canonical action-request schema
/// (16..=200 chars).
String requireIdempotencyKey(Map<String, dynamic> json, String key) {
  final value = requireString(json, key);
  if (value.length < 16 || value.length > 200) {
    throw Spec006Error(
      ErrorCode.validation,
      '$key must be 16..=200 characters',
    );
  }
  return value;
}
