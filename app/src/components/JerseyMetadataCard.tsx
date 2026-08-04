// The jersey (kit) metadata card — the `xjgw` registry row for one kit: Identification / Display
// Flags / Loc String, inline-editable through `set_field`. Asset-id fields (style / art id /
// variants) are read-only: renaming an asset id would also have to rewrite its texture-cache files.
// Extracted from the former Jerseys tab, now hosted inside the Uniforms kit view.

import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import {
  jerseyAssetId,
  JERSEY_STYLE_LABELS,
  LIGHT_COLOR_LABELS,
  LOC_STRING_LABELS,
} from "../lib/teamPresentation";
import type { TeamJerseyDto } from "../bindings/TeamJerseyDto";

export function JerseyMetadataCard({
  projectId,
  record,
}: {
  projectId: number;
  record: TeamJerseyDto;
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const mut = useMutation({
    mutationFn: (v: { field: string; value: number }) =>
      call("set_field", {
        projectId,
        table: "xjgw",
        record: record.record,
        field: v.field,
        intValue: v.value,
      }),
    onSuccess: async () => {
      await qc.invalidateQueries();
      update(await call("project_summary", { projectId }));
    },
  });

  const select = (
    label: string,
    field: string,
    value: number,
    labels: Record<number, string>,
    locked = false,
  ) => (
    <div>
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">{label}</div>
      {locked ? (
        <div className="mt-0.5 font-mono text-xs text-slate-400">{labels[value] ?? value}</div>
      ) : (
        <select
          value={value}
          disabled={mut.isPending}
          onChange={(e) => mut.mutate({ field, value: Number(e.target.value) })}
          className="mt-0.5 w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          {Object.entries(labels).map(([raw, l]) => (
            <option key={raw} value={raw}>
              {l}
            </option>
          ))}
          {!(value in labels) && <option value={value}>{`raw ${value}`}</option>}
        </select>
      )}
    </div>
  );

  const check = (label: string, field: string, value: number, hint?: string) => (
    <label className="flex items-center gap-2 text-xs text-slate-200" title={hint}>
      <input
        type="checkbox"
        checked={value === 1}
        disabled={mut.isPending}
        onChange={(e) => mut.mutate({ field, value: e.target.checked ? 1 : 0 })}
        className="h-3.5 w-3.5 rounded border-slate-600 bg-slate-950 accent-sky-600"
      />
      <span className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">{label}</span>
    </label>
  );

  // Full-width header: every field is a cell in one auto-fill grid, so they pack
  // into as few rows as the current width allows (more columns when wider). The
  // boolean flags share a single full-width row at the bottom.
  return (
    <section className="flex-1 rounded-lg border border-slate-800 bg-slate-900/40 px-4 py-3">
      <div
        className="grid items-start gap-x-4 gap-y-3"
        style={{ gridTemplateColumns: "repeat(auto-fill, minmax(8.5rem, 1fr))" }}
      >
        <div>
          <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
            Asset ID
          </div>
          <div className="mt-0.5 font-mono text-sm leading-tight text-emerald-300">
            {jerseyAssetId(record)}
          </div>
        </div>
        {select("Style", "cVLR", record.style_raw, JERSEY_STYLE_LABELS, true)}
        <ReadOnly label="Art ID" value={String(record.team_art_id)} locked />
        <ReadOnly label="Variant" value={String(record.variant_raw)} locked />
        <ReadOnly label="FE Variant" value={String(record.fe_variant_raw)} locked />
        <NumberCell
          label="Index"
          value={record.jersey_index}
          disabled={mut.isPending}
          onCommit={(v) => mut.mutate({ field: "tYnx", value: v })}
        />
        <NumberCell
          label="Helmet Logo"
          value={record.helmet_logo}
          disabled={mut.isPending}
          onCommit={(v) => mut.mutate({ field: "JpWa", value: v })}
        />
        {select("Light Color", "Ughr", record.is_light_color, LIGHT_COLOR_LABELS)}
        {select("Loc String", "UuZg", record.loc_string_raw, LOC_STRING_LABELS)}

        <div
          className="flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-slate-800 pt-2"
          style={{ gridColumn: "1 / -1" }}
        >
          {check("Helmet #s", "aEfN", record.helmet_numbers)}
          {check("Tie / Laces", "tWcJ", record.has_tie)}
          {check("Is Default", "yFSM", record.is_default ? 1 : 0)}
          {check("Is Lockable", "vInP", record.is_lockable ? 1 : 0)}
        </div>
      </div>

      {mut.error != null && <p className="mt-2 text-[10px] text-rose-400">{String(mut.error)}</p>}
    </section>
  );
}

function ReadOnly({ label, value, locked }: { label: string; value: string; locked?: boolean }) {
  return (
    <div title={locked ? "Locked — asset-id field." : undefined}>
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">{label}</div>
      <div className="mt-0.5 font-mono text-xs text-slate-400">{value}</div>
    </div>
  );
}

/** Small numeric editor committing on blur/Enter. */
function NumberCell({
  label,
  value,
  disabled,
  onCommit,
}: {
  label: string;
  value: number;
  disabled: boolean;
  onCommit: (v: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);
  const commit = () => {
    const n = Number(draft);
    if (Number.isFinite(n) && n !== value) onCommit(n);
    else setDraft(String(value));
  };
  return (
    <div>
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">{label}</div>
      <input
        type="text"
        inputMode="numeric"
        value={draft}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") setDraft(String(value));
        }}
        className="mt-0.5 w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
      />
    </div>
  );
}
