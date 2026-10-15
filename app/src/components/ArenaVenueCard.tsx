import { assetRoot } from "../lib/assetRoot";
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Card, Pill } from "./primitives";
import type { VenueFieldDto } from "../bindings/VenueFieldDto";

/// Phase 2.5 (22 Sep 2026) — the arena table's **venue parameters**, inventoried for the open
/// title. Each field carries a candidate class (research 2026-09-21 pm handoff R51, transcribed
/// into `defs/arena/venue-fields.toml`), whether it is constant across every arena row of this
/// file, and — for the measured 24-bit colour zones — a swatch.
///
/// The card is deliberately modest about what it knows: a class is a grouping candidate from the
/// field's name and value space, **not** a traced runtime effect. Fields Studio will not edit say
/// why, in words: unclassified, constant on this title, or owned by the identity card.
export function ArenaVenueCard({ projectId, arenaId }: { projectId: number; arenaId: number }) {
  const { install, iso } = useProject();
  const installRoot = assetRoot(install) ?? iso?.root ?? null;
  const [showConstants, setShowConstants] = useState(false);
  const q = useQuery({
    queryKey: ["arena-venue", projectId, arenaId, installRoot],
    queryFn: () => call("arena_venue_view", { projectId, arenaId, installRoot: installRoot ?? undefined }),
    enabled: Number.isFinite(arenaId),
    retry: false,
  });
  if (q.isError) {
    return (
      <Card title="Venue parameters">
        <p className="text-xs text-rose-300" data-testid="venue-error">
          {String(q.error)}
        </p>
      </Card>
    );
  }
  const d = q.data;
  if (!d) return null;

  const shown = showConstants ? d.fields : d.fields.filter((f) => f.constant == null);
  const hidden = d.fields.length - shown.length;
  const groups = new Map<string, VenueFieldDto[]>();
  for (const f of shown) {
    const g = groups.get(f.class) ?? [];
    g.push(f);
    groups.set(f.class, g);
  }

  return (
    <Card
      title="Venue parameters"
      subtitle={`${d.fields.length} fields on this title's arena table, measured over its ${d.rows} arena rows. A class groups fields by name and value space — it is not a statement about what the game reads.`}
    >
      <div className="mb-2 flex flex-wrap items-center gap-2 text-xs">
        {d.class_counts.map(([cls, n]) => (
          <Pill key={cls} tone={cls === "unknown" ? "warn" : undefined}>
            {cls.replace(/_/g, " ")} {n}
          </Pill>
        ))}
        {hidden > 0 && (
          <button className="rounded border border-slate-700 px-2 py-0.5 text-slate-300 hover:bg-slate-800" onClick={() => setShowConstants(true)} data-testid="venue-show-constants">
            show {hidden} constant field{hidden === 1 ? "" : "s"}
          </button>
        )}
        {showConstants && hidden === 0 && (
          <button className="rounded border border-slate-700 px-2 py-0.5 text-slate-300 hover:bg-slate-800" onClick={() => setShowConstants(false)} data-testid="venue-hide-constants">
            hide constants
          </button>
        )}
      </div>

      {[...groups.entries()].map(([cls, fields]) => (
        <section key={cls} className="mb-3" data-testid={`venue-group-${cls}`}>
          <h3 className="text-xs font-semibold text-slate-100">
            {cls.replace(/_/g, " ")} <span className="font-normal text-slate-500">— {fields[0].class_note}</span>
          </h3>
          <ul className="mt-1 grid gap-x-4 gap-y-0.5 text-xs sm:grid-cols-2">
            {fields.map((f) => (
              <li key={f.name} className="flex items-baseline gap-2" data-testid={`venue-field-${f.name}`} title={f.refusal ?? undefined}>
                {f.colour && <span className="inline-block h-3 w-3 shrink-0 rounded-sm border border-slate-600" style={{ background: f.colour }} aria-hidden />}
                <span className="font-mono text-slate-300">{f.label}</span>
                <span className="text-slate-100">{f.colour ?? f.value}</span>
                {f.resolved && (
                  <span
                    className={f.resolved_key_lookup === "exact" || f.resolved_key_lookup === "by-hash" ? "text-emerald-300" : "text-amber-200/80"}
                    data-testid={`venue-resolved-${f.name}`}
                    data-key-lookup={f.resolved_key_lookup ?? undefined}
                    title={f.resolved_note ?? undefined}
                  >
                    {f.resolved}
                  </span>
                )}
                {/* Gen7 horns only: the bank the value names opens in the sound-bank browser. Chant
                    and audio-id values never link — no selecting consumer is traced (R97 §1.4). */}
                {f.bank_file && (
                  <Link
                    to={`/app/database/leagues/commentary?bank=${encodeURIComponent(f.bank_file)}`}
                    className="text-sky-300 hover:underline"
                    data-testid={`venue-bank-${f.name}`}
                    title="Open this bank in the sound-bank browser (listing and diagnostic preview only)"
                  >
                    open {f.bank_file}
                  </Link>
                )}
                {!f.resolved && f.resolved_note && (
                  <span className="text-amber-300/80" data-testid={`venue-unresolved-${f.name}`} title={f.resolved_note}>
                    {f.name === "chantaudioid" ? "number only — selection not traced" : "not in the catalogue"}
                  </span>
                )}
                {!f.editable && (
                  <span className="text-slate-500" data-testid={`venue-readonly-${f.name}`}>
                    read-only
                  </span>
                )}
                {f.constant != null && <span className="text-slate-600">constant</span>}
              </li>
            ))}
          </ul>
        </section>
      ))}

      <ul className="mt-2 space-y-0.5 text-[11px] text-slate-500">
        {d.notes.map((n) => (
          <li key={n}>{n}</li>
        ))}
      </ul>
    </Card>
  );
}
