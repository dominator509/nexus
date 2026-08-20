/**
 * ChatComposer - EP-033 M3 shared UI component.
 *
 * Typed chat message input: outbound messages carry a correlation and
 * an idempotency key so retries cannot duplicate sends. The composer
 * produces a validated ChatMessage; message text is data, never
 * command authority (directive D).
 */

import { useState } from "react";
import { ChatMessage, ErrorCode, Spec006Error, type ChatOrigin } from "@nexus/web";

export interface ChatComposerProps {
  conversationId: string;
  correlationId: string;
  origin?: ChatOrigin;
  onSend: (message: ChatMessage) => void;
  onError?: (error: Spec006Error) => void;
}

export function ChatComposer(props: ChatComposerProps): React.ReactElement {
  const { conversationId, correlationId, origin, onSend, onError } = props;
  const [text, setText] = useState("");
  const [messageId, setMessageId] = useState(1);

  function submit(): void {
    try {
      const message = ChatMessage.fromWire({
        message_id: `${conversationId}:msg-${messageId}`,
        conversation_id: conversationId,
        direction: "OUTBOUND",
        origin: origin ?? "HUMAN",
        text,
        correlation_id: correlationId,
        idempotency_key: `${conversationId}:${messageId}`,
        sent_at_unix_ms: Date.now(),
      });
      onSend(message);
      setMessageId((n) => n + 1);
      setText("");
    } catch (error) {
      if (error instanceof Spec006Error) {
        onError?.(error);
        return;
      }
      onError?.(new Spec006Error(ErrorCode.Internal, "Unexpected chat composer failure"));
    }
  }

  return (
    <form
      role="form"
      aria-label="Chat message composer"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <label htmlFor="chat-text">Message</label>
      <textarea
        id="chat-text"
        value={text}
        onChange={(event) => setText(event.target.value)}
        rows={2}
      />
      <button type="submit" disabled={text.length === 0}>
        Send
      </button>
    </form>
  );
}
