/// EP-034 M1 BluetoothDiscovery contract (SPEC-017).
///
/// Bluetooth endpoint discovery and pairing state for nearby Nexus
/// devices (speakers, displays, cameras). The contract layer is
/// provider-neutral; native modules own the platform Bluetooth stack.
library;

import 'errors.dart';
import 'validate.dart';

/// Canonical Bluetooth endpoint kinds.
enum BluetoothEndpointKind {
  speaker('SPEAKER'),
  display('DISPLAY'),
  camera('CAMERA'),
  appliance('APPLIANCE'),
  unknown('UNKNOWN');

  const BluetoothEndpointKind(this.wire);
  final String wire;
}

/// Canonical pairing states.
enum PairingState {
  notPaired('NOT_PAIRED'),
  pairing('PAIRING'),
  paired('PAIRED'),
  failed('FAILED');

  const PairingState(this.wire);
  final String wire;
}

/// BluetoothEndpoint: a discovered nearby Nexus device.
class BluetoothEndpoint {
  const BluetoothEndpoint({
    required this.endpointId,
    required this.localDeviceId,
    required this.kind,
    required this.displayName,
    required this.pairingState,
    this.rssi,
  });

  factory BluetoothEndpoint.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'endpoint_id',
      'local_device_id',
      'kind',
      'display_name',
      'pairing_state',
      'rssi',
    };
    rejectUnknownKeys(json, allowed);
    final rssi = json['rssi'];
    return BluetoothEndpoint(
      endpointId: requireUuid(json, 'endpoint_id'),
      localDeviceId: requireUuid(json, 'local_device_id'),
      kind: requireEnum(
        json,
        'kind',
        BluetoothEndpointKind.values,
        wireOf: (v) => v.wire,
      ),
      displayName: requireString(json, 'display_name'),
      pairingState: requireEnum(
        json,
        'pairing_state',
        PairingState.values,
        wireOf: (v) => v.wire,
      ),
      rssi: rssi is int ? rssi : null,
    );
  }

  final String endpointId;
  final String localDeviceId;
  final BluetoothEndpointKind kind;
  final String displayName;
  final PairingState pairingState;
  final int? rssi;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'endpoint_id': endpointId,
    'local_device_id': localDeviceId,
    'kind': kind.wire,
    'display_name': displayName,
    'pairing_state': pairingState.wire,
    if (rssi != null) 'rssi': rssi,
  };
}

/// BluetoothDiscovery: a discovery session result over real platform
/// scanning. The session binds to the local device and tenant.
class BluetoothDiscovery {
  const BluetoothDiscovery({
    required this.discoveryId,
    required this.deviceId,
    required this.tenantId,
    required this.startedAtUnixS,
    required this.endpoints,
  });

  factory BluetoothDiscovery.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'discovery_id',
      'device_id',
      'tenant_id',
      'started_at_unix_s',
      'endpoints',
    };
    rejectUnknownKeys(json, allowed);
    final started = json['started_at_unix_s'];
    final rawEndpoints = json['endpoints'];
    if (rawEndpoints is! List) {
      throw Spec006Error(ErrorCode.validation, 'endpoints must be a list');
    }
    final endpoints = rawEndpoints
        .map(
          (e) => e is Map<String, dynamic>
              ? BluetoothEndpoint.fromJson(e)
              : throw Spec006Error(
                  ErrorCode.validation,
                  'endpoint entry must be an object',
                ),
        )
        .toList(growable: false);
    return BluetoothDiscovery(
      discoveryId: requireUuid(json, 'discovery_id'),
      deviceId: requireUuid(json, 'device_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      startedAtUnixS: started is int
          ? started
          : throw Spec006Error(
              ErrorCode.validation,
              'started_at_unix_s must be an integer',
            ),
      endpoints: endpoints,
    );
  }

  final String discoveryId;
  final String deviceId;
  final String tenantId;
  final int startedAtUnixS;
  final List<BluetoothEndpoint> endpoints;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'discovery_id': discoveryId,
    'device_id': deviceId,
    'tenant_id': tenantId,
    'started_at_unix_s': startedAtUnixS,
    'endpoints': endpoints.map((e) => e.toJson()).toList(growable: false),
  };
}
