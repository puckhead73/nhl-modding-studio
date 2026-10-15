import { Link, useParams, useSearchParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useIsSaveEditor, useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import { ProfileSections } from "../components/FieldProfile";
import { DetailTabs, type DetailTab } from "../components/DetailTabs";
import { TeamHeader } from "../components/TeamHeader";
import { LinesTab } from "../components/LinesTab";
import { StrategiesTab } from "../components/StrategiesTab";
import { JerseyCaptaincyTab } from "../components/JerseyCaptaincyTab";
import { ContractsTab } from "../components/ContractsTab";
import { DraftPicksTab } from "../components/DraftPicksTab";
import { UniformsTab } from "../components/UniformsTab";
import { LeagueMembershipCard } from "../components/LeagueMembershipCard";
import { TeamScheduleCalendar } from "../components/TeamScheduleCalendar";
import { TeamLogoCard } from "../components/TeamLogoCard";
import { Spinner } from "../components/primitives";
import { useAssetRootArgs } from "../lib/project";
import { useCrumbLabel } from "../lib/breadcrumbs";
import { POS_BADGE } from "../lib/position";
import { ratingToneClass } from "../lib/ratingColor";
import { isAvailable, TEAM_TABS, visibleTabKeys } from "../lib/surfaces";
import type { TeamViewDto } from "../bindings/TeamViewDto";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";

/// The Team detail page — banner + the reference app's tab set. Team Data, Depth Chart, and Lines
/// are live; the remaining tabs are placeholders for the phases that build their systems
/// (strategies, roster editing surfaces, and the Assets tools).
export function TeamDetail() {
  const { id } = useParams<{ id: string }>();
  const [search]=useSearchParams();
  const { project } = useProject();
  const { installDir } = useAssetRootArgs();
  const projectId = project!.project_id;
  const teamId = Number(id);

  const view = useQuery({
    queryKey: ["team", projectId, teamId],
    queryFn: () => call("team_view", { projectId, teamId }),
    enabled: Number.isFinite(teamId),
  });
  // Breadcrumb from the resolved identity (domain-joined name source) — on NHL07's thin index the
  // name lives off the raw `kOtt` row, so read the curated `identity`, not `all_fields`.
  useCrumbLabel(view.data?.name);

  if (view.isLoading) return <Spinner label="Loading…" />;
  if (view.isError) return <p className="text-rose-400">{String(view.error)}</p>;
  const team = view.data;
  if (!team) return <p className="text-sm text-slate-400">Select a team.</p>;

  const tabs: DetailTab[] = [
    {
      key: "team-data",
      label: "Team Data",
      content: <TeamDataTab projectId={projectId} team={team} />,
    },
    ...(project!.game_id === "nhl07" ? [{key:"schedule",label:"Schedule",content:<TeamScheduleCalendar key={`${projectId}/${teamId}`} projectId={projectId} teamId={teamId} league={team.league} />}]:[]),
    {
      key: "depth-chart",
      label: "Depth Chart",
      content: <DepthChartTab projectId={projectId} teamId={teamId} team={team} />,
    },
    {
      key: "lines",
      label: "Lines",
      content: <LinesTab projectId={projectId} teamId={teamId} />,
    },
    {
      key: "strategies",
      label: "Strategies",
      content: <StrategiesTab projectId={projectId} teamId={teamId} />,
    },
    {
      key: "jerseys-captaincy",
      label: "Jersey Numbers & Captaincy",
      content: (
        <JerseyCaptaincyTab
          projectId={projectId}
          teamId={teamId}
          retiredNumbers={team.retired_numbers}
        />
      ),
    },
    {
      key: "contracts",
      label: "Contracts",
      content: <ContractsTab projectId={projectId} teamId={teamId} />,
    },
    {
      key: "draft",
      label: "Draft Picks",
      content: <DraftPicksTab projectId={projectId} teamId={teamId} />,
    },
    // Equipment (glove/helmet zone colors) now lives inside the Uniforms tab — its
    // standalone subtab was removed.
  ];

  // Uniforms is the merged jersey/uniform surface: the `xjgw` jersey-registry metadata editor plus the
  // per-kit textures from the extracted art tree (this replaces the old Jerseys tab). Shown when the
  // schema carries the jersey registry OR the project has an install tree with uniform art.
  if (installDir || isAvailable(project!.surfaces?.jerseys)) {
    tabs.push({
      key: "uniforms",
      label: "Uniforms",
      content: <UniformsTab projectId={projectId} teamId={teamId} />,
      // The kit view uses the whole window width (preview · equipment · textures).
      fullBleed: true,
    });
  }

  // Gate the tab strip on the probed surface contract: a title that ships no strategy sheet, no
  // jersey/equipment tables, or no draft-pick assets omits those tabs entirely (rather than showing
  // a tab that empties out on click). Roster-driven tabs (Depth Chart / Contracts) always show.
  // The install-tree-gated Uniforms tab is not part of the DB-surface contract, so it's kept as-is.
  const visible = visibleTabKeys(TEAM_TABS, project!.surfaces);
  const shownTabs = tabs.filter((t) => t.key === "uniforms" || t.key === "schedule" || visible.has(t.key));

  return (
    <div className="space-y-3">
      {/* Header stays capped for readability; each tab decides its own width
          (Uniforms is full-bleed, the rest cap at max-w-6xl inside DetailTabs). */}
      <div className="max-w-6xl">
        <TeamHeader team={team} />
      </div>
      <DetailTabs key={`${projectId}/${teamId}/${search.get("tab")??""}`} tabs={shownTabs} initial={search.get("tab")??"team-data"} />
    </div>
  );
}

/// The Team Data tab — the reference app's Identity / Classification / Venue / Relations form in
/// the Player Data tab's sectioned 4-column layout, inline-editable. League/Conference/Division
/// resolve through their enum vocabularies (dropdowns); the relation FKs use the team picker
/// (1-indexed, 0 = none); the arena picker writes the 0-indexed StEO id directly. The Venue
/// section disappears on titles without the team→arena link (NHL 10–12).
function TeamDataTab({ projectId, team }: { projectId: number; team: TeamViewDto }) {
  const {project}=useProject();
  const saveEditor = useIsSaveEditor();
  const organizationField=(f:{studio_table:string;studio_field:string}|null|undefined)=>project?.game_id==="nhl07" && f?.studio_table==="HqzU" && ["xMjj","rasz","xgeX"].includes(f.studio_field);
  const profile={...team.profile,sections:team.profile.sections.map(s=>({...s,cells:s.cells.filter(c=>!organizationField(c.field))})).filter(s=>s.cells.length>0)};
  const teamsQ = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });
  const teams = teamsQ.data ?? [];

  // Fields already surfaced as editable cells in the curated sections above — the Misc drawer
  // only carries what ISN'T shown yet, so nothing appears twice on the tab.
  const curated = new Set(
    team.profile.sections
      .flatMap((s) => s.cells)
      // A colour cell owns every component it edits (`_r/_g/_b`), not just its primary field.
      .flatMap((c) => [...(c.field ? [c.field] : []), ...c.color_fields].map((f) => `${f.studio_table}.${f.studio_field}`)),
  );
  const misc = team.all_fields.filter((f) => !organizationField(f) && !curated.has(`${f.studio_table}.${f.studio_field}`));

  // The curated sections come from the domain's per-game field profile. On NHL07's thin team index
  // the name/classification cells route through the joined stock/custom row (so names are editable),
  // and the Venue section is emitted only when the team→arena link exists in this schema.
  return (
    <div className="space-y-6">
      <ProfileSections projectId={projectId} profile={profile} teams={teams} arena={team.arena} teamId={Number(team.team_id)} />
      {team.profile.sections.some(s => s.cells.some(c => c.field?.studio_table === "cubz")) && <p className="text-xs text-slate-400">Arena city changes apply to every team using that arena.</p>}

      <LeagueMembershipCard projectId={projectId} teamId={Number(team.team_id)} />

      {/* Logos are game art; a Save Editor session has no game installation. */}
      {!saveEditor && <TeamLogoCard projectId={projectId} teamId={Number(team.team_id)} />}

      {misc.length > 0 && (
        <details className="rounded-md border border-slate-800 bg-slate-900/40 px-3 py-2">
          <summary className="cursor-pointer text-xs font-semibold text-slate-300">
            Misc fields ({misc.length}) — All fields not presented for editing above, only change if you know what you are doing.
          </summary>
          <div className="mt-2 sm:grid sm:grid-cols-2 sm:gap-x-6">
            {misc.map((f) => (
              <FieldRow key={f.studio_field} projectId={projectId} record={team.record} field={f} />
            ))}
          </div>
        </details>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Depth Chart tab — the reference layout: five position blocks (LW / C / RW on
// top, D / G centered below). Each block lists the team's primary roster
// (players whose main team is this team) above a separator, with "in the
// system" players below it: anyone under this team's contract/rights (`proteam`)
// whose main team (`dXSB`) is elsewhere, plus the AHL affiliate's roster
// (`kOtt.CsxT`) as a fallback for AHL-contract players whose rights field is unset.
// This is a front-end identifier only (Spike S4) — it is NOT a stored roster status.
// ---------------------------------------------------------------------------

const POSITION_BUCKETS = ["LW", "C", "RW", "D", "G"] as const;
type PositionBucket = (typeof POSITION_BUCKETS)[number];

const POSITION_LABELS: Record<PositionBucket, string> = {
  LW: "Left Wing",
  C: "Center",
  RW: "Right Wing",
  D: "Defense",
  G: "Goalie",
};

function bucketFor(label: string): PositionBucket | undefined {
  const t = label.trim().toUpperCase();
  return (POSITION_BUCKETS as readonly string[]).includes(t) ? (t as PositionBucket) : undefined;
}

function playerName(p: PlayerSummaryDto): string {
  if (p.first_name && p.last_name) return `${p.last_name}, ${p.first_name}`;
  return p.last_name || p.first_name || `Player #${p.player_id}`;
}

/** Depth order: inactive players sink, then Overall descending, then name. */
function comparePlayers(a: PlayerSummaryDto, b: PlayerSummaryDto): number {
  const aInactive = a.is_active === false ? 1 : 0;
  const bInactive = b.is_active === false ? 1 : 0;
  if (aInactive !== bInactive) return aInactive - bInactive;
  const aO = a.overall ?? -1;
  const bO = b.overall ?? -1;
  if (aO !== bO) return bO - aO;
  return playerName(a).localeCompare(playerName(b));
}

function DepthChartTab({ projectId, teamId, team }: { projectId: number; teamId: number; team: TeamViewDto }) {
  const players = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  const teamsQ = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });
  // The team's eGlu roster block — the roster source for membership-only teams (Winter
  // Classic / all-star / national), whose players keep another main team.
  const rosterQ = useQuery({
    queryKey: ["teamRoster", projectId, teamId],
    queryFn: () => call("team_roster_ids", { projectId, teamId }),
  });

  if (players.isLoading) return <Spinner label="Loading roster…" />;
  if (players.isError) return <p className="text-rose-400">{String(players.error)}</p>;

  // AHL affiliate from `kOtt.CsxT` (1-indexed team FK; 0 = none).
  const csxt = team.all_fields.find((f) => f.studio_field === "CsxT")?.decoded;
  const affiliateId = csxt != null && csxt > 0 ? csxt - 1 : undefined;
  const teamName = new Map((teamsQ.data ?? []).map((t) => [t.team_id, t.name]));

  const empty = (): Record<PositionBucket, PlayerSummaryDto[]> => ({ LW: [], C: [], RW: [], D: [], G: [] });
  const primary = empty();
  const system = empty();
  // Roster-block members count as primary roster — the main-team pointer alone misses
  // membership-only teams entirely (their players' main team is an NHL club).
  const rosterIds = new Set(rosterQ.data ?? []);
  for (const p of players.data ?? []) {
    const bucket = bucketFor(p.position_label);
    if (!bucket) continue;
    if (p.team_id === teamId || rosterIds.has(p.player_id)) {
      primary[bucket].push(p);
    } else if (
      // "In the system" = under contract to this team but currently playing elsewhere —
      // rights-held prospects on loan anywhere, plus the AHL affiliate's roster as a fallback.
      // `team_id != null` excludes unsigned players (rights-held free agents stay off the chart).
      p.team_id != null &&
      (p.rights_team_id === teamId || (affiliateId !== undefined && p.team_id === affiliateId))
    ) {
      system[bucket].push(p);
    }
  }
  for (const b of POSITION_BUCKETS) {
    primary[b].sort(comparePlayers);
    system[b].sort(comparePlayers);
  }

  const block = (b: PositionBucket) => (
    <PositionBlock bucket={b} primary={primary[b]} system={system[b]} teamName={teamName} />
  );

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        {(["LW", "C", "RW"] as PositionBucket[]).map((b) => (
          <div key={b}>{block(b)}</div>
        ))}
      </div>
      <div className="flex flex-col gap-4 md:flex-row md:justify-center">
        {(["D", "G"] as PositionBucket[]).map((b) => (
          <div key={b} className="md:w-1/3">
            {block(b)}
          </div>
        ))}
      </div>
    </div>
  );
}

function PositionBlock({
  bucket,
  primary,
  system,
  teamName,
}: {
  bucket: PositionBucket;
  primary: PlayerSummaryDto[];
  system: PlayerSummaryDto[];
  teamName: Map<number, string>;
}) {
  const total = primary.length + system.length;
  return (
    <section className="flex h-full flex-col rounded-lg border border-slate-800 bg-slate-900/50">
      <header className="flex items-baseline justify-between border-b border-slate-800 px-3 py-2">
        <h3 className="flex items-center gap-2 text-sm font-semibold text-slate-100">
          <span
            className={
              "inline-flex items-center rounded px-1.5 py-0.5 text-xs font-semibold " +
              (POS_BADGE[bucket] ?? "bg-slate-800 text-slate-300")
            }
          >
            {bucket}
          </span>
          {POSITION_LABELS[bucket]}
        </h3>
        <span className="text-[11px] uppercase tracking-wide text-slate-500">
          {total} {total === 1 ? "player" : "players"}
        </span>
      </header>
      <div className="flex-1 px-3 py-2">
        <DepthList players={primary} emptyLabel="No players" teamName={teamName} />
        <div className="my-2 flex items-center gap-2">
          <span className="h-px flex-1 bg-slate-800" />
          <span className="text-[10px] uppercase tracking-wider text-slate-500">In the system</span>
          <span className="h-px flex-1 bg-slate-800" />
        </div>
        <DepthList
          players={system}
          emptyLabel="No players in the system"
          dim
          showTeam
          teamName={teamName}
        />
      </div>
    </section>
  );
}

function DepthList({
  players,
  emptyLabel,
  dim,
  showTeam,
  teamName,
}: {
  players: PlayerSummaryDto[];
  emptyLabel: string;
  dim?: boolean;
  /** Show each player's current team under the name (the "in the system" rows). */
  showTeam?: boolean;
  teamName: Map<number, string>;
}) {
  if (players.length === 0) {
    return <div className="px-1 py-1 text-xs italic text-slate-600">{emptyLabel}</div>;
  }
  return (
    <ul className="space-y-0.5">
      {players.map((p) => (
        <DepthRow
          key={p.player_id}
          player={p}
          dim={dim}
          team={
            showTeam && p.team_id != null
              ? teamName.get(p.team_id) || "Other league"
              : undefined
          }
        />
      ))}
    </ul>
  );
}

function DepthRow({
  player,
  dim,
  team,
}: {
  player: PlayerSummaryDto;
  dim?: boolean;
  /** Current-team label, rendered under the name for system players. */
  team?: string;
}) {
  const inactive = player.is_active === false;
  const archetype = player.style_label?.trim();
  return (
    <li>
      <Link
        to={`/app/database/players/${player.player_id}`}
        className={
          "flex items-baseline gap-2 rounded px-1 py-0.5 text-xs hover:bg-slate-800/60 " +
          (dim || inactive ? "text-slate-400" : "text-slate-100")
        }
      >
        <span className="inline-flex w-7 shrink-0 justify-end font-mono text-[11px] text-slate-500">
          {player.jersey_number != null ? `#${player.jersey_number}` : "—"}
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <span className="truncate">{playerName(player)}</span>
          {(archetype || team) && (
            <span className="truncate text-[10px] text-slate-500">
              {[archetype, team].filter(Boolean).join(" · ")}
            </span>
          )}
        </span>
        {inactive && (
          <span className="shrink-0 self-center text-[10px] uppercase tracking-wide text-slate-500">
            inactive
          </span>
        )}
        <span
          className={
            "w-7 shrink-0 self-center text-right font-mono text-[11px] font-semibold tabular-nums " +
            ratingToneClass(player.overall)
          }
        >
          {player.overall ?? "—"}
        </span>
      </Link>
    </li>
  );
}
