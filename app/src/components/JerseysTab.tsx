import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { EmptyState, Spinner } from "./primitives";
import {
  compareJerseys,
  jerseyAssetId,
  jerseyStyleLabel,
  lightColorLabel,
  locStringLabel,
  JERSEY_STYLE_LABELS,
  LIGHT_COLOR_LABELS,
  LOC_STRING_LABELS,
} from "../lib/teamPresentation";
import type { TeamJerseyDto } from "../bindings/TeamJerseyDto";

/// The Jerseys tab — the team's `xjgw` registry in the reference layout: searchable jersey list
/// on the left with the metadata card beneath it (Identification / Display Flags / Loc String,
/// inline-editable through `set_field`), and the Textures pane on the right. Texture previews,
/// thumbnails, and the add/delete/rename-asset-id tools operate on the extracted texture cache —
/// they arrive with the Production Asset phase; the pane reserves their place.
export function JerseysTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const q = useQuery({
    queryKey: ["jerseys", projectId, teamId],
    queryFn: () => call("list_team_jerseys", { projectId, teamId }),
  });

  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<number | null>(null);

  const sorted = useMemo(() => [...(q.data ?? [])].sort(compareJerseys), [q.data]);

  // Re-pin the selection when the team changes or the selected record disappears.
  useEffect(() => {
    if (sorted.length === 0) {
      setSelected(null);
      return;
    }
    if (selected === null || !sorted.some((r) => r.record === selected)) {
      setSelected(sorted[0].record);
    }
  }, [sorted, selected]);

  if (q.isLoading) return <Spinner label="Loading jerseys…" />;
  if (q.isError) return <p className="text-rose-400">{String(q.error)}</p>;
  if (sorted.length === 0) {
    return <EmptyState title="No jerseys are registered for this team" />;
  }

  const needle = search.trim().toLowerCase();
  const filtered = needle
    ? sorted.filter((r) =>
        [
          `[${r.jersey_index}]`,
          jerseyAssetId(r),
          jerseyStyleLabel(r.style_raw),
          locStringLabel(r.loc_string_raw),
          lightColorLabel(r.is_light_color),
        ]
          .join(" ")
          .toLowerCase()
          .includes(needle),
      )
    : sorted;
  const current = selected !== null ? sorted.find((r) => r.record === selected) : undefined;
  const pos = current ? sorted.findIndex((r) => r.record === current.record) : -1;

  return (
    <div className="flex min-h-[28rem] flex-wrap gap-3">
      <aside className="flex w-[22rem] shrink-0 flex-col gap-3">
        <div className="flex min-h-0 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
          <header className="border-b border-slate-800 p-3">
            <div className="flex items-center justify-between gap-2">
              <h3 className="text-sm font-semibold text-slate-100">Jerseys</h3>
              <span className="text-[11px] text-slate-500">
                {filtered.length} / {sorted.length}
              </span>
            </div>
            <input
              type="search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search jerseys…"
              className="mt-2 w-full rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-[11px] text-slate-100 placeholder-slate-600"
            />
          </header>
          <ul className="max-h-72 overflow-y-auto">
            {filtered.map((r) => {
              const isSel = r.record === selected;
              return (
                <li key={r.record}>
                  <button
                    type="button"
                    onClick={() => setSelected(r.record)}
                    className={
                      "flex w-full flex-col items-start gap-0.5 border-b border-slate-900 px-3 py-1.5 text-left text-[11px] transition-colors " +
                      (isSel ? "bg-sky-900/40 text-sky-50" : "text-slate-200 hover:bg-slate-800/60")
                    }
                  >
                    <div className="flex w-full items-baseline gap-2">
                      <span className="font-mono text-[10px] text-slate-500">#{r.jersey_index}</span>
                      <span className="font-semibold">{jerseyAssetId(r)}</span>
                      {r.is_default && (
                        <span className="ml-auto rounded bg-emerald-900/40 px-1 text-[9px] uppercase tracking-wide text-emerald-300">
                          Default
                        </span>
                      )}
                    </div>
                    <div className="flex w-full items-baseline gap-2 text-[10px] text-slate-400">
                      <span>{jerseyStyleLabel(r.style_raw)}</span>
                      <span>·</span>
                      <span>{locStringLabel(r.loc_string_raw)}</span>
                    </div>
                  </button>
                </li>
              );
            })}
          </ul>
        </div>
        {current && <JerseyMetadataCard projectId={projectId} record={current} />}
      </aside>
      <section className="min-w-0 flex-1">
        <div className="rounded-lg border border-slate-800 bg-slate-900/40">
          <header className="flex items-center justify-between gap-2 border-b border-slate-800 px-4 py-2">
            <div className="flex items-baseline gap-2">
              <h3 className="text-[11px] font-semibold uppercase tracking-wider text-sky-300">
                Textures
              </h3>
              {current && (
                <span className="font-mono text-[11px] text-slate-500">
                  {jerseyAssetId(current)}
                </span>
              )}
            </div>
            <div className="flex items-center gap-1 text-[11px] tabular-nums text-slate-400">
              <button
                type="button"
                disabled={pos <= 0}
                onClick={() => pos > 0 && setSelected(sorted[pos - 1].record)}
                className="rounded border border-slate-700 px-2 py-0.5 text-slate-200 hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-40"
              >
                ‹ Prev
              </button>
              <span className="px-2">
                Jersey {pos + 1} of {sorted.length}
              </span>
              <button
                type="button"
                disabled={pos < 0 || pos >= sorted.length - 1}
                onClick={() => pos < sorted.length - 1 && setSelected(sorted[pos + 1].record)}
                className="rounded border border-slate-700 px-2 py-0.5 text-slate-200 hover:bg-slate-800 disabled:cursor-not-allowed disabled:opacity-40"
              >
                Next ›
              </button>
            </div>
          </header>
          <div className="px-4 py-6 text-center text-xs text-slate-500">
            Texture previews, nameplate editing, and replace-from-PNG operate on the extracted
            texture cache — they arrive with the Production Asset tools phase. The jersey's asset
            id (<span className="font-mono text-slate-400">{current ? jerseyAssetId(current) : "—"}</span>)
            is what keys those files.
          </div>
        </div>
      </section>
    </div>
  );
}

/** The metadata card: Identification / Display Flags / Loc String, inline-editable. Asset-id
 *  fields (style / art id / variants) are locked — renaming an asset id must also rewrite its
 *  cache files, which arrives with the Assets phase. */
function JerseyMetadataCard({ projectId, record }: { projectId: number; record: TeamJerseyDto }) {
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
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        {label}
      </div>
      {locked ? (
        <div
          className="mt-0.5 font-mono text-xs text-slate-400"
          title="Asset-id field — locked until the Assets phase (renaming must also rewrite cache files)."
        >
          {labels[value] ?? value}
        </div>
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
      <span className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        {label}
      </span>
    </label>
  );

  return (
    <section className="rounded-lg border border-slate-800 bg-slate-900/40">
      <header className="border-b border-slate-800 px-4 py-3">
        <div className="text-[10px] uppercase tracking-wider text-slate-500">Asset ID</div>
        <div className="mt-1 font-mono text-lg leading-tight text-emerald-300">
          {jerseyAssetId(record)}
        </div>
        <div className="mt-0.5 text-[10px] text-slate-500">style_artid_variant</div>
      </header>

      <Subhead>Identification</Subhead>
      <div className="grid grid-cols-2 gap-x-3 gap-y-3 px-4 py-3">
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
      </div>

      <Subhead>Display Flags</Subhead>
      <div className="grid grid-cols-2 gap-x-3 gap-y-3 px-4 py-3">
        {select("Light Color", "Ughr", record.is_light_color, LIGHT_COLOR_LABELS)}
        {check("Helmet #s", "aEfN", record.helmet_numbers)}
        {check("Tie / Laces", "tWcJ", record.has_tie)}
        {check("Is Default", "yFSM", record.is_default ? 1 : 0)}
        {check("Is Lockable", "vInP", record.is_lockable ? 1 : 0)}
      </div>

      <Subhead>Loc String</Subhead>
      <div className="px-4 py-3">
        {select("Loc String", "UuZg", record.loc_string_raw, LOC_STRING_LABELS)}
      </div>

      {mut.error != null && (
        <p className="px-4 pb-2 text-[10px] text-rose-400">{String(mut.error)}</p>
      )}
      <footer className="border-t border-slate-800 px-4 py-2 text-[10px] text-slate-500">
        Asset-id fields (Style / Art ID / Variants) are locked — renaming an asset id must also
        rewrite its texture-cache files, which arrives with the Assets phase.
      </footer>
    </section>
  );
}

function Subhead({ children }: { children: React.ReactNode }) {
  return (
    <div className="border-y border-slate-800 bg-slate-900/30 px-4 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-slate-300">
      {children}
    </div>
  );
}

function ReadOnly({ label, value, locked }: { label: string; value: string; locked?: boolean }) {
  return (
    <div title={locked ? "Locked until the Assets phase." : undefined}>
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        {label}
      </div>
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
      <div className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80">
        {label}
      </div>
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
