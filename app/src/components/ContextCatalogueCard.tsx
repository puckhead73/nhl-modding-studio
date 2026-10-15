import { assetRoot } from "../lib/assetRoot";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Pill } from "./primitives";

/// Phase 1.5 / 2.7 — the **read-only** context listing: per context the trigger that starts its
/// evaluation, the named conditions it tests, and its items — a speech event it speaks or a
/// context trigger it raises. Which context the engine picks, the priority words, the run-once
/// groups and the compiled conditions' meaning are runtime / undecoded and not shown as facts.
///
/// Covers gen7 Xbox 360 / PS3 NHL 11–Legacy (research 2026-09-21 handoff 01 / 16, contract
/// rev. 2) and, since 22 Sep 2026, **PS4 NHL 15–21** through the little-endian `0XTC` walk
/// (research 2026-09-21 pm handoff 15) — the same view either way, with the PS4 note naming the
/// file version and the parts that stay undecoded.
export function ContextCatalogueCard({ projectId }: { projectId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<number | null>(null);
  const q = useQuery({
    queryKey: ["context-catalogue", projectId, installRoot, query],
    queryFn: () => call("context_catalogue", { projectId, installRoot: installRoot!, query, limit: 200 }),
    enabled: installRoot != null,
    retry: false,
  });
  if (installRoot == null) return null;
  if (q.isError) {
    return (
      <Card title="Contexts (which trigger speaks what, read-only)">
        <p className="text-xs text-slate-500" data-testid="context-unavailable">
          {String(q.error)}
        </p>
      </Card>
    );
  }
  const d = q.data;
  return (
    <Card
      title="Contexts (which trigger speaks what, read-only)"
      subtitle={d ? `${d.records} contexts · ${d.triggers} triggers · ${d.condition_names} condition names · ${d.speech_items} speech items, ${d.context_items} raised triggers` : "loading…"}
    >
      <div className="mb-2 flex flex-wrap items-center gap-3 text-xs">
        <input
          data-testid="context-query"
          className="w-72 rounded border border-slate-700 bg-slate-900 px-2 py-1 font-mono text-slate-100"
          placeholder="context / trigger / condition / event name contains…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        {d && (
          <span className="text-slate-500" data-testid="context-matching">
            {d.matching} matching{d.truncated ? ` (first ${d.contexts.length} shown)` : ""}
          </span>
        )}
      </div>
      {d && (
        <ul className="space-y-1 text-xs" data-testid="context-list">
          {d.contexts.map((c) => (
            <li key={c.index} className="rounded border border-slate-800 bg-slate-900/40 px-2 py-1">
              <button type="button" className="flex w-full flex-wrap items-baseline gap-2 text-left" onClick={() => setOpen(open === c.index ? null : c.index)} data-testid={`context-${c.index}`}>
                <span className="font-mono text-slate-200">{c.name}</span>
                <span className="text-slate-500">{c.trigger ? `on ${c.trigger}` : "no trigger (evaluated by another path)"}</span>
                {c.items.map((i, k) => (
                  <Pill key={k} tone={i.kind === "speech" ? "ok" : i.kind === "context" ? "warn" : "neutral"}>
                    {i.kind === "speech" ? "speaks" : i.kind === "context" ? "raises" : "?"} {i.name}
                  </Pill>
                ))}
                <span className="text-slate-500">
                  {c.conditions.length} condition{c.conditions.length === 1 ? "" : "s"}
                </span>
              </button>
              {open === c.index && (
                <div className="mt-1 text-slate-400" data-testid={`context-detail-${c.index}`}>
                  <p>
                    conditions: <span className="font-mono">{c.conditions.length ? c.conditions.join(" · ") : "none"}</span>
                  </p>
                  {c.items.map((i, k) => (
                    <p key={k}>
                      {i.kind === "speech" ? "speaks" : "raises"} <span className="font-mono">{i.name}</span> (event id {i.event_id}){i.params.length ? ` with ${i.params.join(", ")}` : ""}
                    </p>
                  ))}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
      {d && (
        <ul className="mt-2 text-xs text-slate-500">
          {d.notes.map((n, i) => (
            <li key={i}>{n}</li>
          ))}
        </ul>
      )}
    </Card>
  );
}
