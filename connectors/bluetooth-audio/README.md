# nexus-bluetooth-audio

EP-022 M4 Bluetooth audio connector: real D-Bus/BlueZ transport
probing, typed fail-closed `BluetoothEndpointProvider` behavior
(SPEC-012 behavior 7).

## What is real here

- A minimal real D-Bus client (SASL EXTERNAL auth + GetNameOwner) over
  a real Unix socket against the real system bus.
- A probe that resolves `org.bluez` on the system bus. On this build
  host BlueZ is absent, so the probe observes the real
  `NameHasNoOwner` failure and every operation fails closed with a
  typed UNAVAILABLE error, an audit record, and metric counters.
- A pure state machine: duplicate connects conflict, cancellation and
  failure roll back to DISCONNECTED (no partial side effect).
- A default-deny connect policy; only an explicit allowlist grants
  connect.
- Redacted incident records with incident and correlation ids.

## Certification boundary

- Bluetooth connectivity / A2DP transport: DEFERRED to real hardware
  ownership (EP-040 / EP-043). This connector never claims CONNECTED
  without a certified transport.
- BlueZ presence itself: host substrate, observed honestly. When
  org.bluez has no owner, the connector says UNAVAILABLE - it does not
  simulate a device.

## Ops diagnostic

```sh
cargo run -p nexus-bluetooth-audio --bin bluetooth-diag -- status
```

Runs the real system-bus probe and prints structured JSON, e.g.:

- `{"status":"degraded","bus_ok":true,"bluez":"absent",...}` when BlueZ
  is not running;
- `{"status":"degraded","bus_ok":false,...}` when the bus itself is
  unreachable;
- `{"status":"ok","bluez":"present",...}` when BlueZ is running (still
  no transport certification claim).

## Bounded recovery

```sh
cargo run -p nexus-bluetooth-audio --bin bluetooth-diag -- recover
```

Bounded recovery re-probes the real bus and resets in-memory connector
state. It never starts services and never claims connectivity. The
reported action is honest:

- BlueZ absent: install/start BlueZ (`systemctl start bluetooth`) before
  any transport certification;
- Bus unhealthy: diagnose the system bus;
- BlueZ present: transport certification remains deferred.

## System-level diagnostics

```sh
busctl --system call org.freedesktop.DBus /org/freedesktop/DBus \
  org.freedesktop.DBus GetNameOwner s org.bluez
systemctl status bluetooth
```

`GetNameOwner` for an unowned name returns the real
`NameHasNoOwner` error - the same mechanism the connector proves.

## Forced-failure suite

```sh
cargo test --locked -p nexus-bluetooth-audio ep022_failure
```

Real mechanisms exercised: real system bus absence of org.bluez,
unreachable bus socket, silent peer (timeout), garbage peer
(malformed), auth-rejecting peer (authorization), policy denial,
duplicate request, cancellation rollback, malformed device refs, and
payload redaction.
