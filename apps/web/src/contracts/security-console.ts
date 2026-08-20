/**
 * EP-033 M1 SecurityConsole contract (SPEC-004: security).
 *
 * Presents incidents and security events with canonical severity and
 * correlation. The console displays detection state; containment or
 * destructive actions always flow through the typed command dispatch
 * and approval contracts, never through the console rendering layer.
 */

import {
  assertEnum,
  assertObject,
  assertString,
  rejectUnknownFields,
} from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const SEVERITY_LEVELS = [
  "INFO",
  "LOW",
  "MEDIUM",
  "HIGH",
  "CRITICAL",
] as const;
export type SeverityLevel = (typeof SEVERITY_LEVELS)[number];

const INCIDENT_FIELDS = new Set<string>([
  "incident_id",
  "title",
  "severity",
  "status",
  "correlation_id",
]);

export const INCIDENT_STATUSES = [
  "OPEN",
  "TRIAGED",
  "INVESTIGATING",
  "CONTAINED",
  "RESOLVED",
] as const;
export type IncidentStatus = (typeof INCIDENT_STATUSES)[number];

export interface IncidentShape {
  incident_id: string;
  title: string;
  severity: SeverityLevel;
  status: IncidentStatus;
  correlation_id: string;
}

export class SecurityIncident {
  readonly incident_id: string;
  readonly title: string;
  readonly severity: SeverityLevel;
  readonly status: IncidentStatus;
  readonly correlation_id: string;

  private constructor(shape: IncidentShape) {
    this.incident_id = shape.incident_id;
    this.title = shape.title;
    this.severity = shape.severity;
    this.status = shape.status;
    this.correlation_id = shape.correlation_id;
  }

  static fromWire(value: unknown): SecurityIncident {
    const obj = assertObject(value, "SecurityIncident");
    rejectUnknownFields(obj, INCIDENT_FIELDS, "SecurityIncident");
    const incidentId = assertString(obj.incident_id, "incident_id");
    if (incidentId.length === 0) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "incident_id must not be empty",
      );
    }
    return new SecurityIncident({
      incident_id: incidentId,
      title: assertString(obj.title, "title"),
      severity: assertEnum(
        obj.severity,
        new Set<SeverityLevel>(SEVERITY_LEVELS),
        "severity",
      ),
      status: assertEnum(
        obj.status,
        new Set<IncidentStatus>(INCIDENT_STATUSES),
        "status",
      ),
      correlation_id: assertString(obj.correlation_id, "correlation_id"),
    });
  }
}

/** The console is a presentation surface: it never contains a control. */
export class SecurityConsole {
  readonly incidents: ReadonlyArray<SecurityIncident>;
  readonly correlation: string;

  constructor(incidents: ReadonlyArray<SecurityIncident>, correlation: string) {
    this.incidents = [...incidents];
    this.correlation = correlation;
  }

  criticalCount(): number {
    return this.incidents.filter((i) => i.severity === "CRITICAL").length;
  }
}
