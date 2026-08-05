import { useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import { ProfileSections } from "../components/FieldProfile";
import { DetailTabs, type DetailTab } from "../components/DetailTabs";
import { PlayerHeader } from "../components/PlayerHeader";
import { AttributesTab } from "../components/AttributesTab";
import { CareerStatsTab } from "../components/CareerStatsTab";
import { PlayerEquipmentTab } from "../components/PlayerEquipmentTab";
import { Spinner } from "../components/primitives";
import { useCrumbLabel } from "../lib/breadcrumbs";
import { PLAYER_TABS, isAvailable, visibleTabKeys } from "../lib/surfaces";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";

export function PlayerDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const surfaces = project!.surfaces;
  const playerId = Number(id);

  const view = useQuery({
    queryKey: ["player", projectId, playerId],
    queryFn: () => call("player_view", { projectId, playerId }),
    enabled: Number.isFinite(playerId),
  });
  // Don't fetch a surface the schema lacks: skip the career query when this file carries no career
  // tables (roster-save display, or a gen shape without them).
  const careerAvailable = isAvailable(surfaces?.career);
  const career = useQuery({
    queryKey: ["career", projectId, playerId],
    queryFn: () => call("career_stats", { projectId, playerId }),
    enabled: Number.isFinite(playerId) && careerAvailable,
  });
  useCrumbLabel(view.data?.header.full_name);
  if (view.isLoading) return <Spinner label="Loading…" />;
  if (view.isError) return <p className="text-rose-400">{String(view.error)}</p>;
  const player = view.data;
  if (!player) return <p className="text-sm text-slate-400">Select a player.</p>;

  const tabs: DetailTab[] = [
    {
      key: "player-data",
      label: "Player Data",
      content: <PlayerDataTab projectId={projectId} player={player} />,
    },
    {
      key: "career",
      label: "Career Stats",
      content: (
        <CareerStatsTab
          projectId={projectId}
          player={player}
          career={career.data}
          loading={career.isLoading}
        />
      ),
    },
    {
      key: "attributes",
      label: "Attributes",
      content: <AttributesTab projectId={projectId} player={player} />,
    },
    {
      key: "equipment",
      label: "Equipment",
      content: <PlayerEquipmentTab projectId={projectId} player={player} />,
    },
  ];

  // Gate the tab strip on the probed surface contract: omit tabs whose backing schema is absent (an
  // absent feature is never rendered as an empty tab a user can't tell from a bug).
  const visible = visibleTabKeys(PLAYER_TABS, surfaces);
  const shownTabs = tabs.filter((t) => visible.has(t.key));

  return (
    <div className="max-w-6xl space-y-3">
      <PlayerHeader player={player} />
      <DetailTabs tabs={shownTabs} />
    </div>
  );
}

/** The Player Data tab — resolved bio/roster/contract/draft/rookie values in the reference's
 *  sectioned, 4-column layout, inline-editable: enum fields render dropdowns, team FKs render team
 *  pickers, 1-bit flags render checkboxes, and derived fields (cap hit, status) stay read-only. Field
 *  values/labels/vocabularies are all resolved in studio-domain (`header`/`data`/`enum_options`). */
function PlayerDataTab({ projectId, player }: { projectId: number; player: PlayerViewDto }) {
  const teamsQ = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });
  const teams = teamsQ.data ?? [];

  // The curated sections come from the domain's per-game field profile: it emits exactly the fields
  // this title's schema carries (no "not in this game" placeholders) with correct labels/scales.
  // "Additional Teams" is a frontend-only membership editor that belongs in the Roster Data grid.
  return (
    <div className="space-y-6">
      <ProfileSections
        projectId={projectId}
        profile={player.profile}
        teams={teams}
        extras={{ "Roster Data": <AdditionalTeams projectId={projectId} player={player} teams={teams} /> }}
      />

      <details className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2">
        <summary className="cursor-pointer text-xs font-semibold text-slate-300">
          All bio fields ({player.all_fields.length}) — full editor
        </summary>
        <div className="mt-2 sm:grid sm:grid-cols-2 sm:gap-x-6">
          {player.all_fields.map((f) => (
            <FieldRow key={f.studio_field} projectId={projectId} record={player.bio_record} field={f} />
          ))}
        </div>
      </details>
    </div>
  );
}

/** "Additional Teams" — the player's roster spots beyond the canonical Main Team slot (national
 *  teams, all-star / Winter Classic rosters, alternate rosters), as a compact chip row that sits
 *  in the Roster Data grid next to Archetype. One membership row matching Main Team is hidden
 *  (the Main Team picker owns it); the rest show as removable chips, plus an additive "+ Add"
 *  picker filtered to teams the player isn't already on. Add installs the player into a free slot
 *  on the chosen team's roster; ✕ vacates that spot. Neither ever changes the Main Team. */
function AdditionalTeams({
  projectId,
  player,
  teams,
}: {
  projectId: number;
  player: PlayerViewDto;
  teams: { team_id: number; name: string; abbrev: string }[];
}) {
  const qc = useQueryClient();
  const { update } = useProject();
  const [adding, setAdding] = useState(false);
  const refresh = async () => {
    await qc.invalidateQueries();
    update(await call("project_summary", { projectId }));
  };
  const add = useMutation({
    mutationFn: (toTeam: number) =>
      call("add_membership", { projectId, player: player.player_id, toTeam }),
    onSuccess: async () => {
      await refresh();
      setAdding(false);
    },
  });
  const remove = useMutation({
    mutationFn: (fromTeam: number) =>
      call("remove_membership", { projectId, fromTeam, player: player.player_id }),
    onSuccess: refresh,
  });

  // The main team is resolved in the domain (per-title `dXSB` offset, anchored on the roster) and
  // handed over as `header.primary_team_id` — never recompute it here with a hardcoded offset, or
  // early titles (NHL 09/10, where `dXSB` is 0-indexed) mis-split main vs. additional. Exactly one
  // membership row matching it is the canonical slot — hide that one; the rest are additional.
  const mainTeamId = player.header.primary_team_id;
  let mainRowDropped = false;
  const rows = player.memberships.filter((m) => {
    if (!mainRowDropped && mainTeamId !== null && m.team_id === mainTeamId) {
      mainRowDropped = true;
      return false;
    }
    return true;
  });

  const occupied = new Set(player.memberships.map((m) => m.team_id));
  if (mainTeamId !== null) occupied.add(mainTeamId);
  const options = teams
    .filter((t) => !occupied.has(t.team_id))
    .sort((a, b) => a.name.localeCompare(b.name));

  const err = add.error ?? remove.error;
  const busy = add.isPending || remove.isPending;

  return (
    <div className="col-span-2 min-w-0 lg:col-span-3">
      <div
        className="text-[11px] font-semibold uppercase tracking-wide text-sky-300/80"
        title="Extra roster spots beyond the main team — national teams, all-star rosters, alternate rosters. Adding or removing one never changes the main team."
      >
        Additional Teams
        {busy && <span className="ml-1 text-slate-500">…</span>}
      </div>
      <div className="mt-1 flex flex-wrap items-center gap-1.5">
        {rows.length === 0 && !adding && <span className="text-sm text-slate-600">—</span>}
        {rows.map((m, i) => (
          <span
            key={`${m.team_id}-${i}`}
            className="inline-flex items-center gap-1 rounded-full border border-slate-700 bg-slate-900/60 py-0.5 pl-2 pr-1 text-xs"
          >
            <Link
              className="text-sky-400 hover:underline"
              to={`/app/database/teams/${m.team_id}`}
            >
              {m.team_name ?? `Team ${m.team_id}`}
            </Link>
            <button
              type="button"
              onClick={() => remove.mutate(m.team_id)}
              disabled={busy}
              title="Remove this roster spot"
              className="rounded-full px-1 text-slate-500 hover:bg-slate-800 hover:text-rose-400 disabled:opacity-50"
            >
              ✕
            </button>
          </span>
        ))}
        {!adding ? (
          <button
            type="button"
            onClick={() => setAdding(true)}
            disabled={busy || options.length === 0}
            title={options.length === 0 ? "Player is already on every team." : "Add a roster spot"}
            className="rounded-full border border-slate-700 px-2 py-0.5 text-xs text-slate-400 hover:border-sky-700 hover:text-sky-300 disabled:opacity-50"
          >
            + Add
          </button>
        ) : (
          <select
            autoFocus
            defaultValue=""
            disabled={busy}
            onChange={(e) => {
              if (e.target.value !== "") add.mutate(Number(e.target.value));
            }}
            onBlur={() => !add.isPending && setAdding(false)}
            onKeyDown={(e) => e.key === "Escape" && setAdding(false)}
            className="w-48 max-w-full rounded border border-slate-700 bg-slate-950 px-1.5 py-0.5 font-mono text-xs text-slate-100 focus:border-sky-600 focus:outline-none"
          >
            <option value="" disabled>
              Pick a team…
            </option>
            {options.map((t) => (
              <option key={t.team_id} value={t.team_id}>
                {t.name}
                {t.abbrev ? ` (${t.abbrev})` : ""}
              </option>
            ))}
          </select>
        )}
      </div>
      {err != null && <p className="mt-1 text-xs text-rose-400">{String(err)}</p>}
    </div>
  );
}

