import type { ReactNode } from "react";
import { TeamCrest, blockTitle, seasonLabel } from "./HistoricalTeamViews";
import type { HistoricalTeamFranchiseDto } from "../bindings/HistoricalTeamFranchiseDto";
import type { HistoricalTeamBlockDto } from "../bindings/HistoricalTeamBlockDto";

/// The Historical Team banner — the same shape as the EA `TeamHeader`: crest window, name +
/// abbreviation pill, then the classification stack (League · Conference · Division), the parent
/// club, and the roster size. Two extra controls ride the right rail because the historical view is
/// season-scoped where the EA one is not: the season dropdown, and (only when a franchise iced more
/// than one club that season) a constituent picker. The corpus ships no logo art, so the crest is
/// drawn from the club's own colours.
export function HistoricalTeamHeader({
  franchise,
  block,
  season,
  onSeasonChange,
  constituentIndex,
  onConstituentChange,
}: {
  franchise: HistoricalTeamFranchiseDto;
  block: HistoricalTeamBlockDto | null;
  season: number;
  onSeasonChange: (year: number) => void;
  constituentIndex: number;
  onConstituentChange: (index: number) => void;
}) {
  const fullName = block ? blockTitle(franchise.kind, block) : franchise.display_name;
  const abbrev = block?.abbr ?? franchise.abbr;
  const classification = [
    block?.league_abbr ?? franchise.league_abbr,
    block?.conference_name,
    block?.division_name,
  ]
    .filter(Boolean)
    .join(" · ");

  // When the era name differs from the modern franchise name, note the lineage for context.
  const lineage =
    block && fullName !== franchise.display_name ? `${franchise.display_name} franchise` : null;
  const parent = (block?.affiliates ?? []).filter((a) => a.relation === "parent");

  return (
    <div className="flex items-start gap-4 rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <TeamCrest
        name={fullName}
        primary={block?.primary_color ?? null}
        secondary={block?.secondary_color ?? null}
      />

      <div className="min-w-0 flex-1 space-y-1.5">
        <div className="flex items-center gap-2">
          <h1 className="truncate text-2xl font-semibold leading-tight text-slate-50">{fullName}</h1>
          {abbrev && (
            <span className="shrink-0 rounded-full border border-slate-700 bg-slate-800/60 px-2.5 py-0.5 font-mono text-xs font-medium text-teal-300/90">
              {abbrev}
            </span>
          )}
          {franchise.is_defunct && (
            <span className="shrink-0 rounded-full border border-rose-800 bg-rose-900/40 px-2.5 py-0.5 text-xs font-medium text-rose-300">
              Defunct
            </span>
          )}
        </div>

        {classification && <p className="text-xs text-slate-400">{classification}</p>}
        {lineage && <p className="text-xs text-slate-500">{lineage}</p>}
        {parent.length > 0 && (
          <p className="text-xs text-slate-400">
            Parent club:{" "}
            {parent
              .map((a) => a.name ?? a.abbr ?? `Team ${a.src_team_id}`)
              .join(", ")}
          </p>
        )}
        {franchise.notes && <p className="text-xs text-slate-500">{franchise.notes}</p>}
        <p className="text-xs text-slate-400">{block?.roster.length ?? 0} on roster</p>
      </div>

      {/* Right rail: the season scope, plus the constituent picker when several clubs iced under
          this franchise in the selected season. */}
      <div className="flex shrink-0 flex-col items-end gap-2">
        <Picker label="Season" value={String(season)} onChange={(v) => onSeasonChange(Number(v))}>
          {franchise.seasons.map((y) => (
            <option key={y} value={y}>
              {seasonLabel(y)}
            </option>
          ))}
        </Picker>
        {franchise.constituents.length > 1 && (
          <Picker
            label="Club"
            value={String(constituentIndex)}
            onChange={(v) => onConstituentChange(Number(v))}
            hint={`${franchise.constituents.length} clubs iced under this franchise in ${seasonLabel(season)}.`}
          >
            {franchise.constituents.map((c, i) => (
              <option key={c.src_team_id} value={i}>
                {blockTitle(franchise.kind, c)}
              </option>
            ))}
          </Picker>
        )}
      </div>
    </div>
  );
}

function Picker({
  label,
  value,
  onChange,
  hint,
  children,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-2">
        <label className="text-[11px] uppercase tracking-wider text-slate-500">{label}</label>
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          {children}
        </select>
      </div>
      {hint && <span className="max-w-[16rem] text-right text-[10px] text-amber-300/80">{hint}</span>}
    </div>
  );
}
