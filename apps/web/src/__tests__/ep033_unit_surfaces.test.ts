import { describe, expect, it } from "vitest";
import { FleetDevice, FleetView, DEVICE_STATUSES } from "../contracts/fleet-view";
import { SecurityIncident, SecurityConsole, SEVERITY_LEVELS, INCIDENT_STATUSES } from "../contracts/security-console";
import {
  ProviderDisclosure,
  ProviderSettings,
  PROVIDER_ROUTES,
  PROVIDER_CERTIFICATION,
} from "../contracts/provider-settings";
import { AuditFilter, AuditRecord, AuditExplorer } from "../contracts/audit-explorer";
import { ErrorCode, Spec006Error } from "../contracts/errors";

const PRINCIPAL = "00000000-0000-4000-8000-000000000002";

describe("ep033_unit_fleet_view", () => {
  it("constructs fleet devices with canonical status vocabulary", () => {
    const device = FleetDevice.fromWire({
      device_id: "dev-0001",
      name: "edge-01",
      status: "ONLINE",
      correlation: "corr-1",
    });
    expect(device.status).toBe("ONLINE");
    expect([...DEVICE_STATUSES]).toEqual(["ONLINE", "OFFLINE", "DEGRADED", "UNPROVISIONED"]);
  });

  it("rejects duplicate device ids in a fleet", () => {
    const device = FleetDevice.fromWire({
      device_id: "dev-0001",
      name: "edge-01",
      status: "ONLINE",
      correlation: "corr-1",
    });
    expect(() => new FleetView([device, device], "corr-1")).toThrowError(Spec006Error);
  });

  it("counts online devices", () => {
    const fleet = new FleetView(
      [
        FleetDevice.fromWire({ device_id: "a", name: "a", status: "ONLINE", correlation: "c" }),
        FleetDevice.fromWire({ device_id: "b", name: "b", status: "OFFLINE", correlation: "c" }),
      ],
      "corr-1",
    );
    expect(fleet.onlineCount()).toBe(1);
  });
});

describe("ep033_unit_security_console", () => {
  it("constructs incidents with canonical severity and status", () => {
    const incident = SecurityIncident.fromWire({
      incident_id: "inc-0001",
      title: "Port scan detected",
      severity: "HIGH",
      status: "TRIAGED",
      correlation_id: "corr-1",
    });
    expect(incident.severity).toBe("HIGH");
    expect([...SEVERITY_LEVELS]).toEqual(["INFO", "LOW", "MEDIUM", "HIGH", "CRITICAL"]);
    expect([...INCIDENT_STATUSES]).toEqual([
      "OPEN",
      "TRIAGED",
      "INVESTIGATING",
      "CONTAINED",
      "RESOLVED",
    ]);
  });

  it("rejects unknown severity", () => {
    expect(() =>
      SecurityIncident.fromWire({
        incident_id: "inc-1",
        title: "x",
        severity: "CATASTROPHIC",
        status: "OPEN",
        correlation_id: "c",
      }),
    ).toThrowError(Spec006Error);
  });

  it("counts critical incidents for triage presentation", () => {
    const console = new SecurityConsole(
      [
        SecurityIncident.fromWire({ incident_id: "a", title: "a", severity: "CRITICAL", status: "OPEN", correlation_id: "c" }),
        SecurityIncident.fromWire({ incident_id: "b", title: "b", severity: "LOW", status: "OPEN", correlation_id: "c" }),
      ],
      "corr-1",
    );
    expect(console.criticalCount()).toBe(1);
  });
});

describe("ep033_unit_provider_settings", () => {
  it("discloses certification, route, cost, privacy, and egress before activation", () => {
    const disclosure = ProviderDisclosure.fromWire({
      provider_id: "gammu-smsd",
      display_name: "Gammu SMSD",
      route: "SELF_HOSTED",
      certification: "PROVIDER_CERTIFIED",
      cost_description: "self-hosted; local hardware",
      privacy_class: "PERSONAL",
      egress_description: "SMS to carrier gateway",
      correlation: "corr-1",
    });
    expect(disclosure.route).toBe("SELF_HOSTED");
    expect(disclosure.certification).toBe("PROVIDER_CERTIFIED");
    expect(disclosure.activatable).toBe(true);
    expect([...PROVIDER_ROUTES]).toEqual(["SELF_HOSTED", "API", "HYBRID"]);
  });

  it("an uncertified provider is never activatable from the UI", () => {
    const disclosure = ProviderDisclosure.fromWire({
      provider_id: "some-provider",
      display_name: "Some Provider",
      route: "API",
      certification: "NOT_IMPLEMENTED",
      cost_description: "n/a",
      privacy_class: "PERSONAL",
      egress_description: "unknown",
      correlation: "corr-1",
    });
    expect(disclosure.activatable).toBe(false);
  });

  it("certification vocabulary matches the registry status vocabulary", () => {
    expect([...PROVIDER_CERTIFICATION]).toEqual([
      "NOT_IMPLEMENTED",
      "IMPLEMENTED",
      "INTERNAL_CERTIFIED",
      "PROVIDER_CERTIFIED",
      "HARDWARE_CERTIFIED",
      "PRODUCTION_CERTIFIED",
    ]);
  });

  it("rejects duplicate provider ids", () => {
    const disclosure = ProviderDisclosure.fromWire({
      provider_id: "p",
      display_name: "p",
      route: "API",
      certification: "IMPLEMENTED",
      cost_description: "n/a",
      privacy_class: "PERSONAL",
      egress_description: "unknown",
      correlation: "c",
    });
    expect(() => new ProviderSettings([disclosure, disclosure], "corr-1")).toThrowError(Spec006Error);
  });
});

describe("ep033_unit_audit_explorer", () => {
  it("constructs audit records with correlation binding", () => {
    const record = AuditRecord.fromWire({
      audit_id: "aud-0001",
      event_type: "approval.approved",
      source: "approvals",
      correlation_id: "corr-1",
      recorded_at_unix_ms: 1_700_000_000_000,
    });
    expect(record.event_type).toBe("approval.approved");
  });

  it("filters audit records by event type and correlation", () => {
    const record = AuditRecord.fromWire({
      audit_id: "aud-0001",
      event_type: "approval.approved",
      source: "approvals",
      correlation_id: "corr-1",
      recorded_at_unix_ms: 1,
    });
    const explorer = new AuditExplorer([record], "corr-1");
    expect(explorer.filter(new AuditFilter({ event_type: "approval.approved" }))).toHaveLength(1);
    expect(explorer.filter(new AuditFilter({ event_type: "approval.denied" }))).toHaveLength(0);
    expect(explorer.filter(new AuditFilter({ correlation_id: "corr-1" }))).toHaveLength(1);
  });

  it("rejects empty event-type filters", () => {
    expect(() => new AuditFilter({ event_type: "" })).toThrowError(Spec006Error);
  });

  it("rejects duplicate audit ids", () => {
    const record = AuditRecord.fromWire({
      audit_id: "aud-1",
      event_type: "x",
      source: "s",
      correlation_id: "c",
      recorded_at_unix_ms: 1,
    });
    expect(() => new AuditExplorer([record, record], "corr-1")).toThrowError(Spec006Error);
  });
});
