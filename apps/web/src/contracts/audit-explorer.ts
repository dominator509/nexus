/**
 * EP-033 M1 AuditExplorer contract (SPEC-007).
 *
 * Audit exploration is typed: records carry correlation and event
 * vocabulary, and filters are vocabulary-bound. The explorer displays
 * audit history; it never edits it.
 */

import { assertObject, assertString, rejectUnknownFields } from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

const AUDIT_RECORD_FIELDS = new Set<string>([
  "audit_id",
  "event_type",
  "source",
  "correlation_id",
  "recorded_at_unix_ms",
]);

export interface AuditRecordShape {
  audit_id: string;
  event_type: string;
  source: string;
  correlation_id: string;
  recorded_at_unix_ms: number;
}

export class AuditRecord {
  readonly audit_id: string;
  readonly event_type: string;
  readonly source: string;
  readonly correlation_id: string;
  readonly recorded_at_unix_ms: number;

  private constructor(shape: AuditRecordShape) {
    this.audit_id = shape.audit_id;
    this.event_type = shape.event_type;
    this.source = shape.source;
    this.correlation_id = shape.correlation_id;
    this.recorded_at_unix_ms = shape.recorded_at_unix_ms;
  }

  static fromWire(value: unknown): AuditRecord {
    const obj = assertObject(value, "AuditRecord");
    rejectUnknownFields(obj, AUDIT_RECORD_FIELDS, "AuditRecord");
    const auditId = assertString(obj.audit_id, "audit_id");
    if (auditId.length === 0) {
      throw new Spec006Error(ErrorCode.Validation, "audit_id must not be empty");
    }
    return new AuditRecord({
      audit_id: auditId,
      event_type: assertString(obj.event_type, "event_type"),
      source: assertString(obj.source, "source"),
      correlation_id: assertString(obj.correlation_id, "correlation_id"),
      recorded_at_unix_ms:
        typeof obj.recorded_at_unix_ms === "number" ? obj.recorded_at_unix_ms : 0,
    });
  }
}

/** Typed audit filters: event vocabulary and correlation binding. */
export class AuditFilter {
  readonly event_type: string | undefined;
  readonly correlation_id: string | undefined;

  constructor(opts: { event_type?: string; correlation_id?: string } = {}) {
    if (opts.event_type !== undefined && opts.event_type.length === 0) {
      throw new Spec006Error(ErrorCode.Validation, "event_type filter must not be empty");
    }
    this.event_type = opts.event_type;
    this.correlation_id = opts.correlation_id;
  }

  matches(record: AuditRecord): boolean {
    if (this.event_type !== undefined && record.event_type !== this.event_type) {
      return false;
    }
    if (this.correlation_id !== undefined && record.correlation_id !== this.correlation_id) {
      return false;
    }
    return true;
  }
}

export class AuditExplorer {
  readonly records: ReadonlyArray<AuditRecord>;
  readonly correlation: string;

  constructor(records: ReadonlyArray<AuditRecord>, correlation: string) {
    const ids = new Set<string>();
    for (const record of records) {
      if (ids.has(record.audit_id)) {
        throw new Spec006Error(ErrorCode.Conflict, `Duplicate audit record '${record.audit_id}'`);
      }
      ids.add(record.audit_id);
    }
    this.records = [...records];
    this.correlation = correlation;
  }

  filter(filter: AuditFilter): ReadonlyArray<AuditRecord> {
    return this.records.filter((record) => filter.matches(record));
  }
}
