/**
 * EP-033 M1 ChatWorkspace contract (SPEC-004 acceptance: web dashboard
 * supports chat when phone use is impossible).
 *
 * The chat surface is a typed conversation contract: every message
 * carries a correlation, a typed origin (human or agent), and an
 * optional idempotency key so retries cannot duplicate sends. Free-form
 * text is data, never authority: a message can never mint a command
 * (directive D).
 */

import { assertEnum, assertObject, assertString, rejectUnknownFields } from "./validate";
import { ErrorCode, Spec006Error } from "./errors";

export const CHAT_ORIGINS = ["HUMAN", "AGENT"] as const;
export type ChatOrigin = (typeof CHAT_ORIGINS)[number];

export const MESSAGE_DIRECTIONS = ["OUTBOUND", "INBOUND"] as const;
export type MessageDirection = (typeof MESSAGE_DIRECTIONS)[number];

const CHAT_MESSAGE_FIELDS = new Set<string>([
  "message_id",
  "conversation_id",
  "direction",
  "origin",
  "text",
  "correlation_id",
  "idempotency_key",
  "sent_at_unix_ms",
]);

export interface ChatMessageShape {
  message_id: string;
  conversation_id: string;
  direction: MessageDirection;
  origin: ChatOrigin;
  text: string;
  correlation_id: string;
  idempotency_key: string | undefined;
  sent_at_unix_ms: number;
}

/**
 * A validated chat message. Text is bounded (mirroring the canonical
 * notification text bound discipline) and is never interpreted as a
 * command by the UI contract.
 */
export class ChatMessage {
  readonly message_id: string;
  readonly conversation_id: string;
  readonly direction: MessageDirection;
  readonly origin: ChatOrigin;
  readonly text: string;
  readonly correlation_id: string;
  readonly idempotency_key: string | undefined;
  readonly sent_at_unix_ms: number;

  private constructor(shape: ChatMessageShape) {
    this.message_id = shape.message_id;
    this.conversation_id = shape.conversation_id;
    this.direction = shape.direction;
    this.origin = shape.origin;
    this.text = shape.text;
    this.correlation_id = shape.correlation_id;
    this.idempotency_key = shape.idempotency_key;
    this.sent_at_unix_ms = shape.sent_at_unix_ms;
  }

  static fromWire(value: unknown): ChatMessage {
    const obj = assertObject(value, "ChatMessage");
    rejectUnknownFields(obj, CHAT_MESSAGE_FIELDS, "ChatMessage");
    const text = assertString(obj.text, "text");
    if (text.length === 0 || text.length > 4000) {
      throw new Spec006Error(ErrorCode.Validation, "message text must be 1..=4000 characters");
    }
    const messageId = assertString(obj.message_id, "message_id");
    if (messageId.length === 0) {
      throw new Spec006Error(ErrorCode.Validation, "message_id must not be empty");
    }
    const idempotencyKey =
      obj.idempotency_key === undefined ? undefined : assertString(obj.idempotency_key, "idempotency_key");
    if (idempotencyKey !== undefined && idempotencyKey.length < 8) {
      throw new Spec006Error(
        ErrorCode.Validation,
        "idempotency_key must be at least 8 characters",
      );
    }
    return new ChatMessage({
      message_id: messageId,
      conversation_id: assertString(obj.conversation_id, "conversation_id"),
      direction: assertEnum(
        obj.direction,
        new Set<MessageDirection>(MESSAGE_DIRECTIONS),
        "direction",
      ),
      origin: assertEnum(obj.origin, new Set<ChatOrigin>(CHAT_ORIGINS), "origin"),
      text,
      correlation_id: assertString(obj.correlation_id, "correlation_id"),
      idempotency_key: idempotencyKey,
      sent_at_unix_ms:
        typeof obj.sent_at_unix_ms === "number" ? obj.sent_at_unix_ms : Date.now(),
    });
  }

  /** Deduplicate by idempotency key: same key + same text is one send. */
  sameIntent(other: ChatMessage): boolean {
    if (this.idempotency_key === undefined || other.idempotency_key === undefined) {
      return false;
    }
    return (
      this.idempotency_key === other.idempotency_key &&
      this.text === other.text &&
      this.conversation_id === other.conversation_id
    );
  }
}

/** A conversation is bound to a correlation and a bounded message log. */
export class ChatWorkspace {
  readonly conversation_id: string;
  readonly correlation_id: string;
  readonly #messages: Array<ChatMessage> = [];

  constructor(conversationId: string, correlationId: string) {
    if (conversationId.length === 0 || correlationId.length === 0) {
      throw new Spec006Error(ErrorCode.Validation, "conversation and correlation ids required");
    }
    this.conversation_id = conversationId;
    this.correlation_id = correlationId;
  }

  append(message: ChatMessage): void {
    for (const existing of this.#messages) {
      if (existing.sameIntent(message)) {
        throw new Spec006Error(
          ErrorCode.Conflict,
          "Duplicate chat message (same idempotency intent)",
        );
      }
    }
    this.#messages.push(message);
  }

  messages(): ReadonlyArray<ChatMessage> {
    return [...this.#messages];
  }
}
