/**
 * EP-033 M1 ObjectiveView contract.
 *
 * Objectives and their task graph are presented with typed objective
 * ids and canonical lifecycle stages. The view carries the objective
 * correlation so cross-device continuity (LF-005: start by voice,
 * continue in web, approve on mobile, artifact in the same task graph)
 * has an explicit seam; the full journey is owned by M5.
 */

import { assertObject, assertString, rejectUnknownFields } from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const OBJECTIVE_STAGES = [
  "PROPOSED",
  "ACTIVE",
  "AWAITING_APPROVAL",
  "BLOCKED",
  "DONE",
  "CANCELLED",
] as const;
export type ObjectiveStage = (typeof OBJECTIVE_STAGES)[number];

const OBJECTIVE_FIELDS = new Set<string>([
  "objective_id",
  "title",
  "stage",
  "correlation_id",
  "owner_principal_id",
]);

export interface ObjectiveShape {
  objective_id: string;
  title: string;
  stage: ObjectiveStage;
  correlation_id: string;
  owner_principal_id: string;
}

export class ObjectiveView {
  readonly objective_id: string;
  readonly title: string;
  readonly stage: ObjectiveStage;
  readonly correlation_id: string;
  readonly owner_principal_id: string;

  private constructor(shape: ObjectiveShape) {
    this.objective_id = shape.objective_id;
    this.title = shape.title;
    this.stage = shape.stage;
    this.correlation_id = shape.correlation_id;
    this.owner_principal_id = shape.owner_principal_id;
  }

  static fromWire(value: unknown): ObjectiveView {
    const obj = assertObject(value, "ObjectiveView");
    rejectUnknownFields(obj, OBJECTIVE_FIELDS, "ObjectiveView");
    const stage = obj.stage;
    if (
      typeof stage !== "string" ||
      !(OBJECTIVE_STAGES as readonly string[]).includes(stage)
    ) {
      throw new Spec006Error(
        ErrorCode.Vocabulary,
        `Unsupported objective stage '${String(stage)}'`,
      );
    }
    const objectiveId = assertString(obj.objective_id, "objective_id");
    if (objectiveId.length === 0) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "objective_id must not be empty",
      );
    }
    return new ObjectiveView({
      objective_id: objectiveId,
      title: assertString(obj.title, "title"),
      stage: stage as ObjectiveStage,
      correlation_id: assertString(obj.correlation_id, "correlation_id"),
      owner_principal_id: assertString(
        obj.owner_principal_id,
        "owner_principal_id",
      ),
    });
  }

  /**
   * Continuity seam (LF-005): an objective started on another device
   * carries the same correlation, so the web surface can bind to it.
   * The view itself never fabricates progress: stage is displayed, not
   * advanced, by the UI.
   */
  matchesCorrelation(correlationId: string): boolean {
    return this.correlation_id === correlationId;
  }
}

/** A task node inside an objective's graph. */
export class TaskNode {
  readonly task_id: string;
  readonly objective_id: string;
  readonly label: string;
  readonly done: boolean;

  constructor(
    taskId: string,
    objectiveId: string,
    label: string,
    done: boolean,
  ) {
    if (taskId.length === 0 || objectiveId.length === 0 || label.length === 0) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "task ids and label are required",
      );
    }
    this.task_id = taskId;
    this.objective_id = objectiveId;
    this.label = label;
    this.done = done;
  }
}
