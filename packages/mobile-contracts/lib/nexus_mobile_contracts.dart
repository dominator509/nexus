/// Nexus EP-034 mobile core behavior package.
///
/// Pure Dart domain over the nexus_mobile contract layer: approval
/// binding (device AND user), offline cached-policy decisions for
/// low-risk controls, SPEC-006 typed errors, and canonical telemetry
/// redaction. Provider-neutral; native providers are later
/// milestones.
library;

export 'src/behavior/approval_binding.dart';
export 'src/behavior/offline_policy.dart';
export 'src/behavior/telemetry.dart';
