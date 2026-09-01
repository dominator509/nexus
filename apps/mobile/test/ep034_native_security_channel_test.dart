import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nexus_mobile/nexus_mobile.dart';

/// AUD-040 hostile failure suite for the native security channel.
///
/// The production `MethodChannelNativeSecuritySurface` is exercised
/// through the REAL MethodChannel machinery
/// (TestDefaultBinaryMessenger.setMockMethodCallHandler) - the
/// platform side is a CONTROLLED responder that returns
/// platform-shaped payloads, never a mock of the production surface.
/// Every boundary must FAIL CLOSED: an unbound channel, a platform
/// error, a malformed response, a null/absent field, or a
/// non-boolean/non-string value is an honest negative - never an
/// invented success.
void main() {
  const channel = MethodChannel('nexus.mobile/security');
  final surface = MethodChannelNativeSecuritySurface();

  TestWidgetsFlutterBinding.ensureInitialized();

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test(
    'AUD-040: unbound channel reports biometric unavailable (fail closed)',
    () async {
      // No handler registered: MissingPluginException must surface as
      // an honest "no biometric capability", never an exception leak or
      // a fabricated success.
      final capability = await surface.biometricCapability();
      expect(capability.available, isFalse);
    },
  );

  test(
    'AUD-040: unbound channel verification is denied (fail closed)',
    () async {
      final verification = await surface.biometricVerify(reason: 'approve');
      expect(verification.verified, isFalse);
      expect(verification.correlation, isEmpty);
    },
  );

  test('AUD-040: platform error response is an honest denial', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          throw PlatformException(code: 'biometric_unavailable');
        });
    final capability = await surface.biometricCapability();
    expect(capability.available, isFalse);
    final verification = await surface.biometricVerify(reason: 'approve');
    expect(verification.verified, isFalse);
  });

  test('AUD-040: null/absent biometric result is never a success', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async => null);
    final verification = await surface.biometricVerify(reason: 'approve');
    expect(verification.verified, isFalse);
  });

  test(
    'AUD-040: malformed capability response (non-map) fails closed',
    () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async => 'not-a-map');
      final capability = await surface.biometricCapability();
      expect(capability.available, isFalse);
    },
  );

  test(
    'AUD-040: non-boolean available field is treated as unavailable',
    () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(
            channel,
            (call) async => {'available': 'yes'},
          );
      final capability = await surface.biometricCapability();
      expect(capability.available, isFalse);
    },
  );

  test(
    'AUD-040: verification requires verified==true AND a correlation',
    () async {
      // verified true but correlation empty -> denial (cannot bind the
      // resolution to a real platform event).
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            return {'verified': true, 'correlation': ''};
          });
      final noCorrelation = await surface.biometricVerify(reason: 'approve');
      expect(noCorrelation.verified, isFalse);

      // verified false with a correlation -> denial.
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            return {'verified': false, 'correlation': 'abc'};
          });
      final notVerified = await surface.biometricVerify(reason: 'approve');
      expect(notVerified.verified, isFalse);

      // verified true with a real correlation -> success (the ONLY path).
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            return {'verified': true, 'correlation': 'abc-123'};
          });
      final success = await surface.biometricVerify(reason: 'approve');
      expect(success.verified, isTrue);
      expect(success.correlation, 'abc-123');
    },
  );

  test('AUD-040: secure store write requires explicit stored==true', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          return {'stored': true};
        });
    expect(await surface.secureStore(key: 'k', value: 'v'), isTrue);

    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          return {'stored': false};
        });
    expect(await surface.secureStore(key: 'k', value: 'v'), isFalse);

    // Malformed / absent stored field -> false.
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async => {'ok': 1});
    expect(await surface.secureStore(key: 'k', value: 'v'), isFalse);
  });

  test('AUD-040: secure store read fails closed on malformed value', () async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          return {'value': 'secret-value'};
        });
    expect((await surface.secureRead(key: 'k')).value, 'secret-value');

    // Null value is an honest absent entry.
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          return {'value': null};
        });
    expect((await surface.secureRead(key: 'k')).value, isNull);

    // Non-string value is malformed -> absent, never invented.
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          return {'value': 42};
        });
    expect((await surface.secureRead(key: 'k')).value, isNull);
  });

  test('AUD-040: channel contract is the documented name and methods', () {
    expect(kNexusSecurityChannel, 'nexus.mobile/security');
    expect(NexusSecurityMethods.biometricCapability, 'biometricCapability');
    expect(NexusSecurityMethods.biometricVerify, 'biometricVerify');
    expect(NexusSecurityMethods.secureStore, 'secureStore');
    expect(NexusSecurityMethods.secureRead, 'secureRead');
    expect(NexusSecurityMethods.secureDelete, 'secureDelete');
  });
}
