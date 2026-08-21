/// EP-034 M1 device contracts (SPEC-017 canonical vocabulary).
///
/// Binds the canonical `schemas/identity/device-identity.schema.json`
/// vocabulary: ClientDevice, DeviceBinding, PushEndpoint. Deny-unknown
/// on every wire input; unknown device kinds and trust levels are
/// rejected, never defaulted.
library;

import 'errors.dart';
import 'validate.dart';

/// Canonical device kinds (device-identity schema enum).
enum DeviceKind {
  phone('PHONE'),
  tablet('TABLET'),
  desktop('DESKTOP'),
  laptop('LAPTOP'),
  speaker('SPEAKER'),
  camera('CAMERA'),
  display('DISPLAY'),
  server('SERVER'),
  appliance('APPLIANCE'),
  unknown('UNKNOWN');

  const DeviceKind(this.wire);
  final String wire;
}

/// Canonical trust levels (device-identity schema enum).
enum DeviceTrustLevel {
  unverified('UNVERIFIED'),
  local('LOCAL'),
  verified('VERIFIED');

  const DeviceTrustLevel(this.wire);
  final String wire;
}

/// Canonical ClientDevice (SPEC-017; device-identity schema).
class ClientDevice {
  const ClientDevice({
    required this.deviceId,
    required this.tenantId,
    required this.displayName,
    required this.kind,
    required this.trustLevel,
    this.ownerPersonId,
  });

  /// Parses wire input with deny-unknown and canonical enum checks.
  factory ClientDevice.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'device_id',
      'tenant_id',
      'display_name',
      'kind',
      'trust_level',
      'owner_person_id',
    };
    rejectUnknownKeys(json, allowed);
    final owner = json['owner_person_id'];
    return ClientDevice(
      deviceId: requireUuid(json, 'device_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      displayName: requireString(json, 'display_name'),
      kind: requireEnum(json, 'kind', DeviceKind.values, wireOf: (v) => v.wire),
      trustLevel: requireEnum(
        json,
        'trust_level',
        DeviceTrustLevel.values,
        wireOf: (v) => v.wire,
      ),
      ownerPersonId: owner is String ? owner : null,
    );
  }

  final String deviceId;
  final String tenantId;
  final String displayName;
  final DeviceKind kind;
  final DeviceTrustLevel trustLevel;
  final String? ownerPersonId;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'device_id': deviceId,
    'tenant_id': tenantId,
    'display_name': displayName,
    'kind': kind.wire,
    'trust_level': trustLevel.wire,
    if (ownerPersonId != null) 'owner_person_id': ownerPersonId,
  };
}

/// Canonical DeviceBinding: a device bound to a tenant and principal
/// (SPEC-017). High-risk approvals bind to device AND user.
class DeviceBinding {
  const DeviceBinding({
    required this.deviceId,
    required this.tenantId,
    required this.principalId,
    required this.boundAtUnixS,
    this.revoked = false,
  });

  factory DeviceBinding.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'device_id',
      'tenant_id',
      'principal_id',
      'bound_at_unix_s',
      'revoked',
    };
    rejectUnknownKeys(json, allowed);
    final boundAt = json['bound_at_unix_s'];
    final revoked = json['revoked'];
    return DeviceBinding(
      deviceId: requireUuid(json, 'device_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      boundAtUnixS: boundAt is int
          ? boundAt
          : throw Spec006Error(
              ErrorCode.validation,
              'bound_at_unix_s must be an integer',
            ),
      revoked: revoked is bool ? revoked : false,
    );
  }

  final String deviceId;
  final String tenantId;
  final String principalId;
  final int boundAtUnixS;
  final bool revoked;

  /// A revoked binding is terminal for high-risk approvals.
  bool get active => !revoked;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'device_id': deviceId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'bound_at_unix_s': boundAtUnixS,
    'revoked': revoked,
  };
}

/// Canonical PushEndpoint (SPEC-017). Push payloads contain minimal
/// opaque references; sensitive content is fetched after auth.
class PushEndpoint {
  const PushEndpoint({
    required this.endpointId,
    required this.deviceId,
    required this.tenantId,
    this.provider,
    this.token,
  });

  factory PushEndpoint.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'endpoint_id',
      'device_id',
      'tenant_id',
      'provider',
      'token',
    };
    rejectUnknownKeys(json, allowed);
    final provider = json['provider'];
    final token = json['token'];
    return PushEndpoint(
      endpointId: requireUuid(json, 'endpoint_id'),
      deviceId: requireUuid(json, 'device_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      provider: provider is String ? provider : null,
      token: token is String ? token : null,
    );
  }

  final String endpointId;
  final String deviceId;
  final String tenantId;
  final String? provider;
  final String? token;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'endpoint_id': endpointId,
    'device_id': deviceId,
    'tenant_id': tenantId,
    if (provider != null) 'provider': provider,
    if (token != null) 'token': token,
  };
}
