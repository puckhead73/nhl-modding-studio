import { useMemo } from "react";
import { ratingToneClass } from "../lib/ratingColor";
import { POS_BADGE } from "../lib/position";
import type { EaComputedRatingsDto } from "../bindings/EaComputedRatingsDto";

// FHM position-eligibility grade keys — surfaced as pills, not in the raw-attribute grid.
const GRADE_KEYS = ["G", "LD", "RD", "LW", "C", "RW"];

/// Read-only display of a computed EA rating set (from the formula engine), mirroring the EA-DB
/// Attributes look: overall/style headline + FHM position-grade pills + tier-coloured EA rating grid
/// + the raw FHM attributes it was computed from. `compact` hides the raw-attribute disclosure (used
/// in the Formulas live preview).
export function ComputedRatings({ data, compact }: { data: EaComputedRatingsDto; compact?: boolean }) {
  const fhm = useMemo(() => new Map(data.fhm_values.map((v) => [v.key, v.value])), [data.fhm_values]);
  const posLabel =
    data.position === "goalie" ? "Goalie" : data.position === "defenseman" ? "Defenseman" : "Forward";

  // Position pills: goalies show G only; skaters show D (max LD/RD) + LW/C/RW.
  const gradePills: [string, number | undefined][] =
    data.position === "goalie"
      ? [["G", fhm.get("G")]]
      : [
          ["LW", fhm.get("LW")],
          ["C", fhm.get("C")],
          ["RW", fhm.get("RW")],
          ["D", Math.max(fhm.get("LD") ?? 0, fhm.get("RD") ?? 0) || undefined],
        ];

  // Raw FHM attributes = everything except the position grades (grades shown as pills above).
  const rawAttrs = data.fhm_values.filter((v) => !GRADE_KEYS.includes(v.key));

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center gap-4 rounded-lg border border-slate-800 bg-slate-900/50 px-4 py-3">
        <div>
          <div className="text-[10px] uppercase tracking-wide text-slate-500">Overall</div>
          <div className={"text-3xl font-bold tabular-nums " + ratingToneClass(data.overall)}>
            {data.overall ?? "—"}
          </div>
        </div>
        <div>
          <div className="text-[10px] uppercase tracking-wide text-slate-500">Position</div>
          <div className="text-sm text-slate-200">{posLabel}</div>
        </div>
        {data.style_label && (
          <div>
            <div className="text-[10px] uppercase tracking-wide text-slate-500">Style</div>
            <div className="text-sm text-slate-200">{data.style_label}</div>
          </div>
        )}
        <div className="ml-auto flex items-center gap-1.5">
          {gradePills.map(([pos, val]) => (
            <span
              key={pos}
              className={
                "inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] font-semibold " +
                (val != null ? POS_BADGE[pos] ?? "bg-slate-800 text-slate-300" : "bg-slate-800/40 text-slate-600")
              }
              title="FHM position rating"
            >
              {pos}
              <span className="font-mono tabular-nums">{val ?? "—"}</span>
            </span>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-x-8 gap-y-1.5 sm:grid-cols-3 lg:grid-cols-4">
        {data.ratings.map((r) => {
          const isStd = r.scale_min === 36 && r.scale_max === 99;
          return (
            <div
              key={r.ea_key}
              className="flex items-baseline justify-between border-b border-slate-800/50 py-0.5"
            >
              <span className="truncate text-xs text-slate-400" title={r.ea_key}>
                {r.ea_key}
              </span>
              <span
                className={
                  "ml-2 shrink-0 font-mono text-sm font-semibold tabular-nums " +
                  (r.value == null
                    ? "text-slate-600"
                    : isStd
                      ? ratingToneClass(r.value)
                      : "text-slate-200")
                }
                title={isStd ? undefined : `${r.scale_min}..${r.scale_max} scale`}
              >
                {r.value ?? "—"}
              </span>
            </div>
          );
        })}
      </div>

      {!compact && rawAttrs.length > 0 && (
        <details className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2">
          <summary className="cursor-pointer text-xs font-semibold text-slate-300">
            Raw FHM attributes ({rawAttrs.length})
          </summary>
          <div className="mt-2 grid grid-cols-2 gap-x-6 gap-y-0.5 sm:grid-cols-3 lg:grid-cols-4">
            {rawAttrs.map((v) => (
              <div key={v.key} className="flex items-baseline justify-between border-b border-slate-800/40 py-0.5">
                <span className="truncate text-xs text-slate-400" title={v.key}>
                  {v.key}
                </span>
                <span className="ml-2 shrink-0 font-mono text-xs tabular-nums text-slate-300">
                  {v.value}
                </span>
              </div>
            ))}
          </div>
        </details>
      )}

      <p className="text-[11px] text-slate-500">
        Computed from FHM attributes via the active formula profile. Standard ratings are on EA's
        36–99 scale (tier-coloured); biases (0–15) and goalie consistency (0–2) use their own scales.
      </p>
    </div>
  );
}
