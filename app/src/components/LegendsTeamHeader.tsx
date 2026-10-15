import { TeamCrest, blockTitle, seasonLabel } from "./HistoricalTeamViews";
import type { HistoricalTeamFranchiseDto } from "../bindings/HistoricalTeamFranchiseDto";
import type { HistoricalTeamBlockDto } from "../bindings/HistoricalTeamBlockDto";

/// The Legends team banner — the same shape as the EA `TeamHeader` and the Historical one: crest
/// window, name + abbreviation pill, classification stack, roster size.
///
/// The right rail carries **Season Identity**, and that label is the whole point of this page: the
/// dropdown picks which era the built team *wears* — its colours, name/location, and (once the logo
/// database is back) its crest — and nothing else. It does not re-scope the roster the way the
/// Historical team view's season dropdown does; the Legends roster is assembled separately from
/// calculated OVR, so identity and roster are deliberately independent knobs.
export function LegendsTeamHeader({
  franchise,
  block,
  identitySeason,
  onIdentitySeasonChange,
  rosterCount,
}: {
  franchise: HistoricalTeamFranchiseDto;
  block: HistoricalTeamBlockDto | null;
  identitySeason: number;
  onIdentitySeasonChange: (year: number) => void;
  rosterCount: number;
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

  // When the worn era name differs from the modern franchise name, name the lineage.
  const lineage =
    block && fullName !== franchise.display_name ? `${franchise.display_name} franchise` : null;

  return (
    <div className="flex items-start gap-4 rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <TeamCrest
        name={fullName}
        primary={block?.primary_color ?? null}
        secondary={block?.secondary_color ?? null}
        title={`${fullName} — club colours (the logo database isn't wired up yet)`}
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
        <p className="text-xs text-slate-400">
          Wearing the {seasonLabel(identitySeason)} identity · {rosterCount} on roster
        </p>
      </div>

      {/* Right rail: Season Identity — the era this build wears, not a roster filter. */}
      <div className="flex shrink-0 flex-col items-end gap-1">
        <div className="flex items-center gap-2">
          <label
            className="text-[11px] uppercase tracking-wider text-slate-500"
            title="Which era's colours, name/location and crest the built team wears. It does not change the roster."
          >
            Season Identity
          </label>
          <select
            value={identitySeason}
            onChange={(e) => onIdentitySeasonChange(Number(e.target.value))}
            className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1 text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
          >
            {franchise.seasons.map((y) => (
              <option key={y} value={y}>
                {seasonLabel(y)}
              </option>
            ))}
          </select>
        </div>
        <span className="max-w-[18rem] text-right text-[10px] text-slate-500">
          Sets the colours, name and crest only — never the roster.
        </span>
      </div>
    </div>
  );
}
