import { describe, expect, it } from "vitest";
import { ChatMessage, ChatWorkspace, CHAT_ORIGINS, MESSAGE_DIRECTIONS } from "../contracts/chat-workspace";
import { ErrorCode, Spec006Error } from "../contracts/errors";

function messageWire(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    message_id: "msg-0001",
    conversation_id: "conv-0001",
    direction: "OUTBOUND",
    origin: "HUMAN",
    text: "Turn off the kitchen lights",
    correlation_id: "corr-0001",
    idempotency_key: "send-00000001",
    sent_at_unix_ms: 1_700_000_000_000,
    ...overrides,
  };
}

describe("ep033_unit_chat_workspace", () => {
  it("constructs a typed chat message", () => {
    const message = ChatMessage.fromWire(messageWire());
    expect(message.origin).toBe("HUMAN");
    expect(message.direction).toBe("OUTBOUND");
    expect(message.text).toBe("Turn off the kitchen lights");
  });

  it("exposes canonical chat origins and directions", () => {
    expect([...CHAT_ORIGINS]).toEqual(["HUMAN", "AGENT"]);
    expect([...MESSAGE_DIRECTIONS]).toEqual(["OUTBOUND", "INBOUND"]);
  });

  it("rejects empty or oversized message text", () => {
    expect(() => ChatMessage.fromWire(messageWire({ text: "" }))).toThrowError(Spec006Error);
    expect(() => ChatMessage.fromWire(messageWire({ text: "x".repeat(4001) }))).toThrowError(
      Spec006Error,
    );
  });

  it("rejects short idempotency keys", () => {
    expect(() => ChatMessage.fromWire(messageWire({ idempotency_key: "tiny" }))).toThrowError(
      Spec006Error,
    );
  });

  it("rejects unknown fields", () => {
    expect(() => ChatMessage.fromWire(messageWire({ action_name: "lights-off" }))).toThrowError(
      Spec006Error,
    );
  });

  it("treats message text as data, never as command authority", () => {
    const message = ChatMessage.fromWire(
      messageWire({ text: "capability home.lights.set approve=true" }),
    );
    // No command field exists on a chat message: the message cannot
    // mint authority.
    expect("capability_id" in message).toBe(false);
    expect("approval_class" in message).toBe(false);
  });

  it("deduplicates identical idempotent sends in a conversation", () => {
    const workspace = new ChatWorkspace("conv-0001", "corr-0001");
    const first = ChatMessage.fromWire(messageWire());
    const duplicate = ChatMessage.fromWire(messageWire());
    workspace.append(first);
    expect(() => workspace.append(duplicate)).toThrowError(Spec006Error);
    try {
      workspace.append(duplicate);
      expect.unreachable();
    } catch (error) {
      expect((error as Spec006Error).code).toBe(ErrorCode.Conflict);
    }
  });

  it("allows distinct messages in the same conversation", () => {
    const workspace = new ChatWorkspace("conv-0001", "corr-0001");
    workspace.append(ChatMessage.fromWire(messageWire()));
    workspace.append(
      ChatMessage.fromWire(messageWire({ message_id: "msg-0002", idempotency_key: "send-00000002" })),
    );
    expect(workspace.messages()).toHaveLength(2);
  });

  it("supports chat when phone use is impossible (web surface contract)", () => {
    // SPEC-004 acceptance: web dashboard supports chat. The contract
    // surface is origin-neutral: HUMAN or AGENT messages flow through
    // the same typed envelope regardless of device.
    const agent = ChatMessage.fromWire(messageWire({ origin: "AGENT", direction: "INBOUND" }));
    expect(agent.origin).toBe("AGENT");
  });
});
