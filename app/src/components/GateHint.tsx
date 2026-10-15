// ONB-002 — the dependency gate for asset edits that need extracted containers. A disabled
// control cannot reliably receive hover or focus, so the reason is rendered as visible helper
// text with an id the control references through `aria-describedby`, and repeated as `title`.
// This is a per-dependency gate: only edits whose actual inputs are missing use it.

import { useId, type ReactNode } from "react";

export const EXTRACTION_GATE_REASON =
  "Container extraction is required to edit this asset. Run Extract game containers in Settings → Tools.";

/** Wraps a gated control: `reason` is shown beside it and linked for assistive technology. */
export function GateHint({ reason, children }: { reason: string | null; children: (describedBy: string | undefined, disabled: boolean) => ReactNode }) {
  const id = useId();
  if (!reason) return <>{children(undefined, false)}</>;
  return (
    <span className="inline-flex flex-col gap-1">
      <span title={reason}>{children(id, true)}</span>
      <span id={id} role="note" className="text-[11px] text-amber-300">
        {reason}
      </span>
    </span>
  );
}

/** The "not extracted here" pill with the actionable reason on hover and for assistive technology. */
export function NotExtractedPill() {
  return (
    <span title={EXTRACTION_GATE_REASON} aria-label={`not extracted here. ${EXTRACTION_GATE_REASON}`} className="inline-flex items-center rounded-full border border-slate-600 bg-slate-800/60 px-2 py-0.5 text-[11px] text-slate-400">
      not extracted here
    </span>
  );
}
