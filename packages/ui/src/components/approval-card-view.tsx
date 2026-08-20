/**
 * ApprovalCardView - EP-033 M3 shared UI component.
 *
 * Renders the canonical approval class verbatim (directive L): the UI
 * never collapses HUMAN/STRONG_HUMAN/FOUR_EYES/POLICY into a generic
 * Approve boolean. FOUR_EYES surfaces the two-distinct-principals
 * requirement. Accessibility contract: labeled regions, non-color
 * status, keyboard operable action.
 */

import type { ApprovalCard } from "@nexus/web";

export interface ApprovalCardViewProps {
  card: ApprovalCard;
  state: string;
  distinctApprovers: ReadonlyArray<string>;
}

export function ApprovalCardView(props: ApprovalCardViewProps): React.ReactElement {
  const { card, state, distinctApprovers } = props;
  const fourEyes = card.requiresTwoPrincipals();
  return (
    <section aria-label={`Approval ${card.approval_id}`} data-approval-id={card.approval_id}>
      <h3>{card.action_label}</h3>
      <dl>
        <dt>Class</dt>
        <dd data-approval-class={card.approval_class}>{card.approval_class}</dd>
        <dt>Target</dt>
        <dd>{card.target}</dd>
        <dt>Risk</dt>
        <dd>{card.risk}</dd>
        <dt>External effects</dt>
        <dd>{card.external_effects}</dd>
        <dt>Reversibility</dt>
        <dd>{card.reversibility}</dd>
        <dt>Requester</dt>
        <dd>{card.requester_id}</dd>
        <dt>Expires</dt>
        <dd>{new Date(card.expires_at_unix_s * 1000).toISOString()}</dd>
      </dl>
      <p role="status" aria-live="polite">
        State: {state}
      </p>
      {fourEyes ? (
        <p data-four-eyes="true" role="note">
          This action requires approval from two distinct principals.
          {distinctApprovers.length > 0
            ? ` Approvals recorded: ${distinctApprovers.length}.`
            : ""}
        </p>
      ) : null}
    </section>
  );
}
