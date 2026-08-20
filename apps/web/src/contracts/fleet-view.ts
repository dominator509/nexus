/**
 * EP-033 M1 FleetView contract (SPEC-004: fleet).
 *
 * The fleet view presents devices bound to the canonical
 * device-identity vocabulary. Device status is observed state; the UI
 * never claims a device capability the backend has not certified.
 */

import { assertEnum, assertObject, assertString, rejectUnknownFields } from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const DEVICE_STATUSES = [
  "ONLINE",
  "OFFLINE",
  "DEGRADED",
  "UNPROVISIONED",
] as const;
export type DeviceStatus = (typeof DEVICE_STATUSES)[number];

const FLEET_DEVICE_FIELDS = new Set<string>([
  "device_id",
  "name",
  "status",
  "correlation",
]);

export interface FleetDeviceShape {
  device_id: string;
  name: string;
  status: DeviceStatus;
  correlation: string;
}

export class FleetDevice {
  readonly device_id: string;
  readonly name: string;
  readonly status: DeviceStatus;
  readonly correlation: string;

  private constructor(shape: FleetDeviceShape) {
    this.device_id = shape.device_id;
    this.name = shape.name;
    this.status = shape.status;
    this.correlation = shape.correlation;
  }

  static fromWire(value: unknown): FleetDevice {
    const obj = assertObject(value, "FleetDevice");
    rejectUnknownFields(obj, FLEET_DEVICE_FIELDS, "FleetDevice");
    const deviceId = assertString(obj.device_id, "device_id");
    if (deviceId.length === 0) {
      throw new Spec006Error(ErrorCode.Validation, "device_id must not be empty");
    }
    return new FleetDevice({
      device_id: deviceId,
      name: assertString(obj.name, "name"),
      status: assertEnum(obj.status, new Set<DeviceStatus>(DEVICE_STATUSES), "status"),
      correlation: assertString(obj.correlation, "correlation"),
    });
  }
}

export class FleetView {
  readonly devices: ReadonlyArray<FleetDevice>;
  readonly correlation: string;

  constructor(devices: ReadonlyArray<FleetDevice>, correlation: string) {
    const ids = new Set<string>();
    for (const device of devices) {
      if (ids.has(device.device_id)) {
        throw new Spec006Error(ErrorCode.Conflict, `Duplicate device '${device.device_id}'`);
      }
      ids.add(device.device_id);
    }
    this.devices = [...devices];
    this.correlation = correlation;
  }

  onlineCount(): number {
    return this.devices.filter((d) => d.status === "ONLINE").length;
  }
}
