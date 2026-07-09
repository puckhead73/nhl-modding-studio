import { Link } from "react-router-dom";
import type { TeamViewDto } from "../bindings/TeamViewDto";
import type { FieldViewDto } from "../bindings/FieldViewDto";

/// The Team detail banner — logo window, name + abbreviation pill, then the classification stack:
/// League · Conference · Division, the venue (linked to the arena page), and the roster size.
/// Mirrors the Player banner: everything renders from already-resolved field views (enum labels
/// from the league/conference/division vocabularies), no raw ids as primary text. The header is a
/// clean summary — editing lives in the Team Data tab.
export function TeamHeader({ team }: { team: TeamViewDto }) {
  const byId = (id: string): FieldViewDto | undefined =>
    team.all_fields.find((f) => f.studio_field === id);
  const text = (id: string) => byId(id)?.display.trim() || undefined;
  const label = (id: string) => {
    const f = byId(id);
    return f?.enum_label ?? undefined;
  };

  const fullName = text("YmkJ") || text("QNTI") || `Team #${team.team_id}`;
  const abbrev = text("rbPR");
  const inactive = byId("kKYN")?.decoded === 0;

  const classification = [label("xMjj"), label("OwTu"), label("CGMK")]
    .filter(Boolean)
    .join(" · ");

  return (
    <div className="flex items-start gap-4 rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <LogoTile name={fullName} />

      <div className="min-w-0 flex-1 space-y-1.5">
        <div className="flex items-center gap-2">
          <h1 className="truncate text-2xl font-semibold leading-tight text-slate-50">{fullName}</h1>
          {abbrev && (
            <span className="shrink-0 rounded-full border border-slate-700 bg-slate-800/60 px-2.5 py-0.5 text-xs font-medium text-slate-300">
              {abbrev}
            </span>
          )}
          {inactive && (
            <span className="shrink-0 rounded-full border border-rose-800 bg-rose-900/40 px-2.5 py-0.5 text-xs font-medium text-rose-300">
              Inactive
            </span>
          )}
        </div>

        {classification && <p className="text-xs text-slate-400">{classification}</p>}

        {team.arena && (
          <p className="text-xs text-slate-400">
            <Link
              className="text-sky-400 hover:underline"
              to={`/app/database/arenas/${team.arena.arena_id}`}
            >
              {team.arena.name || `Arena ${team.arena.arena_id}`}
            </Link>
            {team.arena.city && ` · ${team.arena.city}`}
          </p>
        )}

        <p className="text-xs text-slate-400">{team.roster_count} on roster</p>
      </div>
    </div>
  );
}

function LogoTile({ name }: { name: string }) {
  return (
    <div
      className="grid h-28 w-28 shrink-0 place-items-center overflow-hidden rounded-md border border-slate-700 bg-slate-800/70"
      title={`${name} — logo preview arrives with the Assets tools`}
    >
      {/* No logo pipeline yet (Assets phase) — a neutral shield placeholder. */}
      <svg viewBox="0 0 24 24" className="h-14 w-14 text-slate-600" fill="currentColor" aria-hidden>
        <path d="M12 2 4 5v6c0 5 3.4 9.4 8 11 4.6-1.6 8-6 8-11V5l-8-3Z" />
      </svg>
    </div>
  );
}
