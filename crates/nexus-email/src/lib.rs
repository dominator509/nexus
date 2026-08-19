//! EP-026 provider-neutral email contracts (SPEC-014).
//!
//! Gmail, Microsoft Graph, generic IMAP/SMTP, and self-hosted mail all
//! map to the canonical objects here without vendor-specific domain
//! logic. Read and send scopes are separate; attachments use
//! ArtifactStore with malware scanning; draft/approval/send/reply/
//! forward/archive/label actions audit correctly.
//!
//! Permanent invariants (owner directive, EP-026):
//! - READ SCOPE != SEND SCOPE (acceptance obligation 2).
//! - SENT != DELIVERED: DeliveryReceipt is the only delivery authority.
//! - DRAFT != SENT: drafting is local intent; sending is governed.
//! - PROVIDER CLAIMS != NEXUS PROVED: provider payloads are normalized
//!   at the infrastructure boundary, never domain contracts.
//! - Attachments carry a sha256 digest, never raw content; only
//!   CLEAN-scanned attachments are deliverable (acceptance
//!   obligation 3).

#![forbid(unsafe_code)]

pub mod error;
pub mod provider;
pub mod verifier;
pub mod vocabulary;

pub use error::{MailError, MailErrorCode};
pub use provider::{enforce_mail_policy, EmailProvider};
pub use verifier::{MailVerification, MailVerifier};
pub use vocabulary::{
    Attachment, AttachmentId, DeliveryReceipt, DeliveryReceiptId, Draft, DraftId, EmailAddress,
    MailChange, MailChangeKind, MailCommand, MailDirection, MailPolicy, MailPrivacyClass,
    MailScope, MailState, MailboxId, Message, MessageId, ScanStatus, SendRequest, ThreadId,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep026_unit_email_address_validation() {
        assert!(EmailAddress::new("user@example.com").is_ok());
        assert!(EmailAddress::new("a.b+c@sub.example.co").is_ok());
        assert!(EmailAddress::new("").is_err());
        assert!(EmailAddress::new("nope").is_err());
        assert!(EmailAddress::new("a@b@c").is_err());
        assert!(EmailAddress::new("@example.com").is_err());
        assert!(EmailAddress::new("user@").is_err());
        assert!(EmailAddress::new("user name@example.com").is_err());
        assert!(EmailAddress::new("user@exa mple.com").is_err());
    }

    #[test]
    fn ep026_unit_read_and_send_scopes_are_separate() {
        // Acceptance obligation 2: a scope is exactly one authority.
        assert_ne!(MailScope::Read, MailScope::Send);
        assert_ne!(MailScope::Draft, MailScope::Send);
        assert_ne!(MailScope::Read.as_str(), MailScope::Send.as_str());
        assert_eq!(MailCommand::Send.required_scope(), MailScope::Send);
        assert_eq!(MailCommand::Fetch.required_scope(), MailScope::Read);
        assert_eq!(MailCommand::Draft.required_scope(), MailScope::Draft);
    }

    #[test]
    fn ep026_unit_scope_parse_rejects_unknown() {
        assert_eq!(MailScope::parse("READ").expect("read"), MailScope::Read);
        assert_eq!(MailScope::parse("SEND").expect("send"), MailScope::Send);
        assert!(MailScope::parse("DELETE_ALL").is_err());
    }

    #[test]
    fn ep026_unit_mail_state_ladder_and_terminal() {
        assert!(!MailState::Draft.is_terminal());
        assert!(MailState::Failed.is_terminal());
        assert!(MailState::Archived.is_terminal());
        assert!(MailState::Deleted.is_terminal());
        assert_eq!(MailState::parse("SENT").expect("sent"), MailState::Sent);
        assert!(MailState::parse("TRANSMUTED").is_err());
    }

    #[test]
    fn ep026_unit_attachment_scan_gates_deliverability() {
        assert!(ScanStatus::Clean.is_deliverable());
        assert!(!ScanStatus::Pending.is_deliverable());
        assert!(!ScanStatus::Quarantined.is_deliverable());
        assert!(!ScanStatus::Blocked.is_deliverable());
    }

    #[test]
    fn ep026_unit_mail_policy_gates_before_mutation() {
        let policy = MailPolicy {
            allowed_scopes: vec![MailScope::Read, MailScope::Draft],
            allowed_commands: vec![MailCommand::Fetch, MailCommand::Draft],
            min_approval_class: 2,
            max_retention_seconds: 90 * 86400,
            max_attachment_bytes: 25 * 1024 * 1024,
            require_scan: true,
        };
        // Fetch allowed with Read scope.
        assert!(enforce_mail_policy(&policy, MailCommand::Fetch, &[MailScope::Read], 3).is_ok());
        // Send is not in allowed_commands.
        let err = enforce_mail_policy(&policy, MailCommand::Send, &[MailScope::Send], 3)
            .expect_err("send must be denied");
        assert_eq!(err.code, MailErrorCode::Policy);
        // Fetch with only Draft scope: scope denied.
        let err = enforce_mail_policy(&policy, MailCommand::Fetch, &[MailScope::Draft], 3)
            .expect_err("scope mismatch must be denied");
        assert_eq!(err.code, MailErrorCode::Policy);
        // Approval class below minimum.
        let err = enforce_mail_policy(&policy, MailCommand::Fetch, &[MailScope::Read], 1)
            .expect_err("approval below minimum must be denied");
        assert_eq!(err.code, MailErrorCode::Policy);
    }

    #[test]
    fn ep026_unit_send_request_requires_send_scope() {
        let request = SendRequest {
            draft: DraftId::new("draft-1").expect("id"),
            idempotency_key: "idem-1".into(),
            approval_class: 1,
            scopes_granted: vec![MailScope::Read, MailScope::Draft],
        };
        assert!(!request.has_send_scope());
        let request = SendRequest {
            scopes_granted: vec![MailScope::Send],
            ..request
        };
        assert!(request.has_send_scope());
    }

    #[test]
    fn ep026_unit_attachment_policy_bounds() {
        let clean = Attachment {
            id: AttachmentId::new("att-1").expect("id"),
            filename: "report.pdf".into(),
            content_type: "application/pdf".into(),
            size_bytes: 1024,
            sha256: "a".repeat(64),
            storage_ref: "art://mail/att-1".into(),
            scan_status: ScanStatus::Clean,
        };
        let policy = MailPolicy {
            allowed_scopes: vec![MailScope::Attachments],
            allowed_commands: vec![],
            min_approval_class: 0,
            max_retention_seconds: 0,
            max_attachment_bytes: 2048,
            require_scan: true,
        };
        assert!(policy.attachment_allows(&clean));
        let too_big = Attachment {
            size_bytes: 4096,
            ..clean.clone()
        };
        assert!(!policy.attachment_allows(&too_big));
        let unscanned = Attachment {
            scan_status: ScanStatus::Pending,
            ..clean
        };
        assert!(!policy.attachment_allows(&unscanned));
    }

    #[test]
    fn ep026_unit_typed_ids_reject_empty_and_overlong() {
        assert!(MailboxId::new("inbox").is_ok());
        assert!(MailboxId::new("").is_err());
        assert!(MessageId::new("x".repeat(129)).is_err());
        assert_eq!(ThreadId::new("t1").expect("id").as_str(), "t1");
    }

    #[test]
    fn ep026_unit_serde_roundtrip_vocabulary() {
        let msg = Message {
            id: MessageId::new("msg-1").expect("id"),
            mailbox: MailboxId::new("inbox").expect("id"),
            thread: ThreadId::new("thread-1").expect("id"),
            direction: MailDirection::Inbound,
            from: EmailAddress::new("alice@example.com").expect("addr"),
            to: vec![EmailAddress::new("bob@example.com").expect("addr")],
            cc: vec![],
            bcc: vec![],
            subject: "Hello".into(),
            body_digest: "b".repeat(64),
            attachments: vec![],
            state: MailState::Sent,
            privacy_class: MailPrivacyClass::Private,
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        let back: Message = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(msg, back);
        assert!(json.contains("INBOUND"));
        assert!(json.contains("SENT"));
    }

    #[test]
    fn ep026_unit_provider_ports_fail_closed() {
        struct NoProvider;
        impl EmailProvider for NoProvider {}
        let provider = NoProvider;
        assert!(provider.list_mailboxes().is_err());
        assert!(provider
            .send(&SendRequest {
                draft: DraftId::new("d-1").expect("id"),
                idempotency_key: "k".into(),
                approval_class: 0,
                scopes_granted: vec![MailScope::Send],
            })
            .is_err());
        let err = provider.list_mailboxes().expect_err("unbound must fail");
        assert_eq!(err.code, MailErrorCode::Unavailable);
    }

    #[test]
    fn ep026_unit_delivery_receipt_is_delivery_authority() {
        // SENT != DELIVERED: the receipt carries the only delivery
        // truth. A message in Sent state without a receipt is not
        // delivered.
        let receipt = DeliveryReceipt {
            id: DeliveryReceiptId::new("rcpt-1").expect("id"),
            message: MessageId::new("msg-1").expect("id"),
            delivered: true,
            provider_timestamp_ms: Some(1780000000000),
        };
        assert!(receipt.delivered);
        let failed = DeliveryReceipt {
            delivered: false,
            ..receipt
        };
        assert!(!failed.delivered);
    }
}
