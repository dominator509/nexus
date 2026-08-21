/// EP-034 M1 PushInbox contract (SPEC-017).
///
/// Push payloads contain minimal opaque references; sensitive content
/// is fetched after authentication (SPEC-017 behavior 8). The inbox
/// never renders secret-bearing push content as domain state.
library;

import 'errors.dart';
import 'validate.dart';

/// Canonical push notification kinds.
enum PushNotificationKind {
  approvalRequest('APPROVAL_REQUEST'),
  event('EVENT'),
  incident('INCIDENT'),
  message('MESSAGE');

  const PushNotificationKind(this.wire);
  final String wire;
}

/// Canonical inbox states.
enum PushInboxState {
  unread('UNREAD'),
  read('READ'),
  actioned('ACTIONED'),
  dismissed('DISMISSED');

  const PushInboxState(this.wire);
  final String wire;
}

/// PushNotification: a minimal opaque push reference (SPEC-017
/// behavior 8). The payload contains no secret; the client fetches
/// sensitive content after auth using the opaque reference.
class PushNotification {
  const PushNotification({
    required this.notificationId,
    required this.tenantId,
    required this.principalId,
    required this.deviceId,
    required this.kind,
    required this.opaqueRef,
    required this.state,
    required this.receivedAtUnixS,
  });

  factory PushNotification.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'notification_id',
      'tenant_id',
      'principal_id',
      'device_id',
      'kind',
      'opaque_ref',
      'state',
      'received_at_unix_s',
    };
    rejectUnknownKeys(json, allowed);
    final received = json['received_at_unix_s'];
    return PushNotification(
      notificationId: requireUuid(json, 'notification_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      deviceId: requireUuid(json, 'device_id'),
      kind: requireEnum(
        json,
        'kind',
        PushNotificationKind.values,
        wireOf: (v) => v.wire,
      ),
      opaqueRef: requireString(json, 'opaque_ref'),
      state: requireEnum(
        json,
        'state',
        PushInboxState.values,
        wireOf: (v) => v.wire,
      ),
      receivedAtUnixS: received is int
          ? received
          : throw Spec006Error(
              ErrorCode.validation,
              'received_at_unix_s must be an integer',
            ),
    );
  }

  final String notificationId;
  final String tenantId;
  final String principalId;
  final String deviceId;
  final PushNotificationKind kind;
  final String opaqueRef;
  final PushInboxState state;
  final int receivedAtUnixS;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'notification_id': notificationId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'device_id': deviceId,
    'kind': kind.wire,
    'opaque_ref': opaqueRef,
    'state': state.wire,
    'received_at_unix_s': receivedAtUnixS,
  };
}

/// PushInbox: the device-bound inbox of opaque push references.
class PushInbox {
  const PushInbox({
    required this.deviceId,
    required this.tenantId,
    required this.principalId,
    required this.notifications,
  });

  factory PushInbox.fromJson(Map<String, dynamic> json) {
    const allowed = <String>{
      'device_id',
      'tenant_id',
      'principal_id',
      'notifications',
    };
    rejectUnknownKeys(json, allowed);
    final raw = json['notifications'];
    if (raw is! List) {
      throw Spec006Error(ErrorCode.validation, 'notifications must be a list');
    }
    final notifications = raw
        .map(
          (e) => e is Map<String, dynamic>
              ? PushNotification.fromJson(e)
              : throw Spec006Error(
                  ErrorCode.validation,
                  'notification must be an object',
                ),
        )
        .toList(growable: false);
    return PushInbox(
      deviceId: requireUuid(json, 'device_id'),
      tenantId: requireUuid(json, 'tenant_id'),
      principalId: requireUuid(json, 'principal_id'),
      notifications: notifications,
    );
  }

  final String deviceId;
  final String tenantId;
  final String principalId;
  final List<PushNotification> notifications;

  Map<String, dynamic> toJson() => <String, dynamic>{
    'device_id': deviceId,
    'tenant_id': tenantId,
    'principal_id': principalId,
    'notifications': notifications
        .map((n) => n.toJson())
        .toList(growable: false),
  };
}
