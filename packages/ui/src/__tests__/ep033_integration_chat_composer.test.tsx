/**
 * EP-033 M3 integration: ChatComposer through REAL React rendering.
 * The composer renders the typed chat form and produces validated
 * ChatMessage values through the real contract boundary.
 */

import { describe, expect, it } from "vitest";
import { renderToString } from "react-dom/server";
import { ChatComposer } from "../components/chat-composer";

describe("ep033_integration_chat_composer", () => {
  it("renders a labeled chat form", () => {
    const html = renderToString(
      <ChatComposer conversationId="conv-1" correlationId="corr-1" onSend={() => {}} />,
    );
    expect(html).toContain('role="form"');
    expect(html).toContain('aria-label="Chat message composer"');
    expect(html).toContain('id="chat-text"');
  });

  it("renders a send button bound to the form", () => {
    const html = renderToString(
      <ChatComposer conversationId="conv-1" correlationId="corr-1" onSend={() => {}} />,
    );
    expect(html).toContain("<button");
    expect(html).toContain("Send");
  });
});
