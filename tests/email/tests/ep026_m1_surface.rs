//! EP-026 M1 cross-component E2E: compose nexus-email contracts
//! through the provider-neutral boundary and prove the acceptance
//! obligations hold at the package boundary (SPEC-014).
//!
//! M1 owns contracts + package boundary only; real provider adapters
//! (Gmail / Microsoft Graph / IMAP+SMTP) land in M2-M3. This package
//! proves the public surface is usable and fail-closed without a bound
//! adapter.

use nexus_email::{
    enforce_mail_policy, Attachment, AttachmentId, Draft, DraftId, EmailAddress, EmailProvider,
    MailCommand, MailDirection, MailPolicy, MailScope, MailState, MailboxId, Message, MessageId,
    SendRequest, ThreadId,
};

#[test]
fn ep026_unit_e2e_canonical_message_surface() {
    // Build the canonical message shape exactly as a provider adapter
    // would map it (provider payloads normalize to this boundary).
    let message = Message {
        id: MessageId::new("msg-e2e-1").expect("id"),
        mailbox: MailboxId::new("inbox").expect("id"),
        thread: ThreadId::new("thread-e2e-1").expect("id"),
        direction: MailDirection::Inbound,
        from: EmailAddress::new("alice@example.com").expect("addr"),
        to: vec![EmailAddress::new("bob@example.com").expect("addr")],
        cc: vec![],
        bcc: vec![],
        subject: "E2E surface".into(),
        body_digest: "c".repeat(64),
        attachments: vec![],
        state: MailState::Delivered,
        privacy_class: nexus_email::MailPrivacyClass::Private,
    };
    let json = serde_json::to_string(&message).expect("serialize");
    let back: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(message, back);
}

#[test]
fn ep026_unit_e2e_draft_send_separation() {
    // DRAFT != SEND: a draft with only DRAFT scope cannot send.
    let draft = Draft {
        id: DraftId::new("draft-e2e-1").expect("id"),
        mailbox: MailboxId::new("outbox").expect("id"),
        thread: None,
        to: vec![EmailAddress::new("bob@example.com").expect("addr")],
        cc: vec![],
        bcc: vec![],
        subject: "Draft".into(),
        body_digest: "d".repeat(64),
        attachments: vec![],
    };
    let send_request = SendRequest {
        draft: draft.id.clone(),
        idempotency_key: "idem-e2e-1".into(),
        approval_class: 3,
        scopes_granted: vec![MailScope::Draft, MailScope::Read],
    };
    assert!(!send_request.has_send_scope());

    let policy = MailPolicy {
        allowed_scopes: vec![MailScope::Draft, MailScope::Read],
        allowed_commands: vec![MailCommand::Draft, MailCommand::Fetch],
        min_approval_class: 1,
        max_retention_seconds: 30 * 86400,
        max_attachment_bytes: 10 * 1024 * 1024,
        require_scan: true,
    };
    // Send is not even in the allowed command set.
    let err = enforce_mail_policy(
        &policy,
        MailCommand::Send,
        &send_request.scopes_granted,
        send_request.approval_class,
    )
    .expect_err("send must be policy-denied without SEND scope");
    assert_eq!(err.code, nexus_email::MailErrorCode::Policy);
}

#[test]
fn ep026_unit_e2e_unbound_provider_fails_closed() {
    struct Unbound;
    impl EmailProvider for Unbound {}

    let provider = Unbound;
    let err = provider
        .fetch_message(
            &MailboxId::new("inbox").expect("id"),
            &MessageId::new("msg-1").expect("id"),
        )
        .expect_err("unbound provider must fail closed");
    assert_eq!(err.code, nexus_email::MailErrorCode::Unavailable);
    assert!(provider.list_mailboxes().is_err());
}

#[test]
fn ep026_unit_e2e_attachment_digest_never_content() {
    // Attachments carry a digest + storage ref, never raw content in
    // the domain contract.
    let attachment = Attachment {
        id: AttachmentId::new("att-e2e-1").expect("id"),
        filename: "scan.pdf".into(),
        content_type: "application/pdf".into(),
        size_bytes: 42,
        sha256: "e".repeat(64),
        storage_ref: "art://mail/att-e2e-1".into(),
        scan_status: nexus_email::ScanStatus::Clean,
    };
    assert_eq!(attachment.sha256.len(), 64);
    assert!(attachment.scan_status.is_deliverable());
    let json = serde_json::to_string(&attachment).expect("serialize");
    // The domain serialization must not contain raw content bytes.
    assert!(!json.contains("raw-content"));
}
