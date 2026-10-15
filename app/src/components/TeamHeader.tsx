import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject, useAssetRootArgs } from "../lib/project";
import type { TeamViewDto } from "../bindings/TeamViewDto";
import type { FieldViewDto } from "../bindings/FieldViewDto";

/// The Team detail banner — logo window, name + abbreviation pill, then the classification stack:
/// League · Conference · Division, the venue (linked to the arena page), and the roster size.
/// Mirrors the Player banner: the league text comes from the server's per-title league catalog
/// (`league_label`: a resolved label, a localization key shown as a key, or `League code N`), and
/// conference/division labels appear only where the title anchors them — otherwise the raw group
/// id is shown as a group id, never as a borrowed label. Editing lives in the Team Data tab.
export function TeamHeader({ team }: { team: TeamViewDto }) {
  const byId = (id: string): FieldViewDto | undefined =>
    team.all_fields.find((f) => f.studio_field === id);
  const text = (id: string) => byId(id)?.display.trim() || undefined;
  const groupText = (label: string | null, id: string, what: string) => {
    if (label) return label;
    const raw = byId(id)?.decoded;
    return raw == null ? undefined : `${what} group ${raw}`;
  };

  const fullName = team.name || `Team #${team.team_id}`;
  const abbrev = text("rbPR");
  const inactive = byId("kKYN")?.decoded === 0;

  const classification = [
    team.league_label ?? undefined,
    groupText(team.conference_label, "OwTu", "Conference"),
    groupText(team.division_label, "CGMK", "Division"),
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <div className="flex items-start gap-4 rounded-lg border border-slate-800 bg-gradient-to-r from-slate-900/80 to-slate-900/30 p-4">
      <TeamLogo teamId={team.team_id} name={fullName} />

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

/// The team logo window. Resolves the team's `kOtt.lOnr` art id server-side and shows the decoded,
/// content-cropped logo scaled so its longest edge is 95% of the tile; falls back to a neutral shield
/// while loading or when the title ships no logo (or the project was opened without an install tree).
function TeamLogo({ teamId, name }: { teamId: number; name: string }) {
  const { project } = useProject();
  const { installDir, rootArgs } = useAssetRootArgs();
  const logo = useQuery({
    queryKey: ["teamLogo", project?.project_id, installDir, teamId],
    queryFn: () => call("team_logo", { projectId: project!.project_id, roots: rootArgs, teamId }),
    enabled: !!project && !!installDir,
    staleTime: Infinity,
  });

  return (
    <div
      className="grid h-28 w-28 shrink-0 place-items-center overflow-hidden rounded-md border border-slate-700 bg-slate-800/70"
      title={logo.data ? `${name} logo` : `${name} — no logo art found for this title`}
    >
      {logo.data ? (
        <img
          src={logo.data}
          alt={`${name} logo`}
          className="h-[95%] w-[95%] object-contain"
        />
      ) : (
        <svg viewBox="0 0 24 24" className="h-14 w-14 text-slate-600" fill="currentColor" aria-hidden>
          <path d="M12 2 4 5v6c0 5 3.4 9.4 8 11 4.6-1.6 8-6 8-11V5l-8-3Z" />
        </svg>
      )}
    </div>
  );
}
