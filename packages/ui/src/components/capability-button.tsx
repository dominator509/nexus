/**
 * CapabilityButton - EP-033 M3 shared UI component.
 *
 * Renders a capability action ONLY when the capability is known and
 * visible (directive E). Unknown capabilities render nothing (fail
 * closed, never a fabricated panel). A visible-but-unauthorized
 * capability renders disabled: VISIBLE != AUTHORIZED (directive D).
 * Accessibility contract: labeled button with keyboard operability and
 * reduced-motion safety.
 */

import type { PresentedCapability } from "@nexus/web";

export interface CapabilityButtonProps {
  capability: PresentedCapability;
  label: string;
  disabledReason?: string;
}

export function CapabilityButton(
  props: CapabilityButtonProps,
): React.ReactElement | null {
  const { capability, label, disabledReason } = props;
  if (!capability.visible) {
    return null;
  }
  if (!capability.operational) {
    return null;
  }
  const disabled = !capability.invocable;
  return (
    <button
      type="button"
      data-capability={capability.capability_id}
      aria-label={label}
      aria-disabled={disabled || undefined}
      disabled={disabled}
      title={disabled ? (disabledReason ?? "Not authorized") : undefined}
    >
      {label}
    </button>
  );
}
