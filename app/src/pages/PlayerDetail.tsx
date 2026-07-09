import { useState, type ReactNode } from "react";
import { Link, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import { EditableField, ReadonlyField, TeamSelect } from "../components/EditableField";
import { DetailTabs, type DetailTab } from "../components/DetailTabs";
import { PlayerHeader } from "../components/PlayerHeader";
import { AttributesTab } from "../components/AttributesTab";
import { CareerStatsTab } from "../components/CareerStatsTab";
import { EmptyState, Spinner } from "../components/primitives";
import { useCrumbLabel } from "../lib/breadcrumbs";
import type { PlayerViewDto } from "../bindings/PlayerViewDto";

export function PlayerDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const playerId = Number(id);

  const view = useQuery({
    queryKey: ["player", projectId, playerId],
    queryFn: () => call("player_view", { projectId, playerId }),
    enabled: Number.isFinite(playerId),
  });
  const career = useQuery({
    queryKey: ["career", projectId, playerId],
    queryFn: () => call("career_stats", { projectId, playerId }),
    enabled: Number.isFinite(playerId),
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
      key: "portrait",
      label: "Portrait",
      content: (
        <EmptyState
          title="Portrait — preview only"
          hint="Portrait preview and DDS↔PNG replace arrive with the Production Asset tools in a later phase."
        />
      ),
    },
  ];

  return (
    <div className="max-w-6xl space-y-3">
      <PlayerHeader player={player} />
      <DetailTabs tabs={tabs} />
    </div>
  );
}

/** A titled block whose cells flow in a 4-column grid (matching the reference Player Data layout). */
function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3 className="mb-3 border-b border-slate-800 pb-1 text-[11px] font-bold uppercase tracking-wider text-slate-400">
        {title}
      </h3>
      <div className="grid grid-cols-2 items-start gap-x-8 gap-y-3 sm:grid-cols-3 lg:grid-cols-4">
        {children}
      </div>
    </section>
  );
}

/** The Player Data tab — resolved bio/roster/contract/draft/rookie values in the reference's
 *  sectioned, 4-column layout, inline-editable: enum fields render dropdowns, team FKs render team
 *  pickers, 1-bit flags render checkboxes, and derived fields (cap hit, status) stay read-only. Field
 *  values/labels/vocabularies are all resolved in studio-domain (`header`/`data`/`enum_options`). */
function PlayerDataTab({ projectId, player }: { projectId: number; player: PlayerViewDto }) {
  const { project } = useProject();
  const caps = project!.capabilities;
  const teamsQ = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });
  const teams = teamsQ.data ?? [];
  const rec = player.bio_record;
  const d = player.data;
  const byId = (id: string) => player.all_fields.find((f) => f.studio_field === id);

  // An editable field cell by studio id; renders nothing if this title's schema lacks the field.
  const F = ({ id, label }: { id: string; label: string }) => {
    const f = byId(id);
    return f ? <EditableField projectId={projectId} record={rec} field={f} label={label} /> : null;
  };
  // A team-FK cell by studio id; hidden (not read-only) when this title's schema lacks the field.
  const Team = ({ id, label, noneLabel }: { id: string; label: string; noneLabel?: string }) => {
    const f = byId(id);
    return f ? (
      <TeamSelect
        projectId={projectId}
        record={rec}
        field={f}
        teams={teams}
        label={label}
        noneLabel={noneLabel}
      />
    ) : null;
  };
  const yesNo = (b: boolean | null) => (b == null ? "" : b ? "Yes" : "No");

  // The whole Draft Data section disappears when this game predates draft data (NHL 10 has none).
  const hasDraft = caps.draft_team || caps.draft_year || caps.draft_round || caps.draft_pick;
  // What this game's database doesn't carry — disclosed once at the bottom instead of leaving the
  // user to wonder whether a blank cell is a bug.
  const notInThisGame = [
    !caps.junior_team && "Junior Team",
    !caps.player_style && "Archetype",
    !caps.year_ufa && "Year UFA",
    !caps.rookie_ahl && "AHL Rookie",
    !caps.rookie_chl && "CHL Rookie",
    !caps.draft_team && "Draft Team",
    !caps.draft_year && "Draft Year",
    !caps.draft_round && "Draft Round",
    !caps.draft_pick && "Draft Position",
  ].filter((x): x is string => Boolean(x));

  return (
    <div className="space-y-6">
      <Section title="Biographical Data">
        <F id="HdeP" label="First Name" />
        <F id="QbMR" label="Last Name" />
        <F id="ypBQ" label="Height" />
        <F id="sNZW" label="Weight" />
        <F id="qFnd" label="Birth Year" />
        <F id="JKLp" label="Birth Month" />
        <F id="Kswi" label="Birth Day" />
        <F id="MFzJ" label="Birthplace" />
        <F id="QvxI" label="Birth State" />
        <F id="Lelh" label="Nationality" />
        <F id="vjla" label="Position" />
        <F id="GRkp" label="Handedness" />
      </Section>

      <Section title="Roster Data">
        <F id="sVRt" label="Jersey Number" />
        <Team id="dXSB" label="Main Team" />
        <Team id="ZbNy" label="Junior Team" />
        {/* Real player active flag: `kKYN` = bio `active` (1-bit) — renders as a checkbox. Distinct
            from `tEok`/`c_potential`, which the prior app mislabelled "Active". */}
        <F id="kKYN" label="Active" />
        {caps.player_style && (
          <ReadonlyField label="Archetype">{player.header.style_label ?? ""}</ReadonlyField>
        )}
        <AdditionalTeams projectId={projectId} player={player} teams={teams} />
      </Section>

      <Section title="Contract Data">
        <Team id="dbBW" label="Pro Team" noneLabel="(none — No NHL Rights)" />
        <ReadonlyField label="Cap Hit">{d.cap_hit ?? ""}</ReadonlyField>
        <F id="IhDG" label="Length Of Contract" />
        <ReadonlyField label="Contract Status">{d.contract_status ?? ""}</ReadonlyField>
        <F id="GKKz" label="Year UFA" />
        <F id="LoVD" label="Two-Way" />
        <ReadonlyField label="Multi-Year">{yesNo(d.is_multi_year)}</ReadonlyField>
      </Section>

      <Section title="Rookie Status">
        <F id="dCnv" label="NHL Rookie" />
        <F id="ywRO" label="AHL Rookie" />
        <F id="BQVU" label="CHL Rookie" />
      </Section>

      {hasDraft && (
        <Section title="Draft Data">
          <Team id="vgWu" label="Draft Team" noneLabel="(undrafted)" />
          <F id="YKzW" label="Draft Year" />
          <F id="ccjU" label="Draft Round" />
          <F id="tTfW" label="Draft Position" />
        </Section>
      )}

      {notInThisGame.length > 0 && (
        <p className="text-xs text-slate-500">
          Not part of this game's database (hidden): {notInThisGame.join(", ")}.
        </p>
      )}

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

  // Main Team = ubPc.dXSB − 1 (0 = free agent). Exactly one membership row matching it is the
  // canonical slot — hide that one; any duplicate beyond it counts as an additional membership.
  const dxsb = player.all_fields.find((f) => f.studio_field === "dXSB")?.decoded ?? 0;
  const mainTeamId = dxsb > 0 ? dxsb - 1 : null;
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

