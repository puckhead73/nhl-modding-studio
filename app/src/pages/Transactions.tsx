import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { queueFieldWrite } from "../lib/fieldWrites";
import { refreshFields } from "../lib/useFieldEdit";
import { useProject } from "../lib/project";
import { Pill, Spinner } from "../components/primitives";
import { ratingToneClass } from "../lib/ratingColor";
import type { DraftPickDto } from "../bindings/DraftPickDto";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";
import type { TeamSummaryDto } from "../bindings/TeamSummaryDto";

// ---------------------------------------------------------------------------
// Transactions — the reference app's NHLViewNG-style two-team trade panel.
//
// Two mirrored columns, each picking a league + team and listing that team's roster, the global
// free-agent pool, or its future draft picks. The action buttons act on the selection relative to
// the OTHER column's team, composing the audited roster ops:
//   Move        → move_player(player, thisTeam, otherTeam)   (block-coherent slot move)
//   Copy        → clone_player(player, otherTeam)            (duplicate cluster, kept here too)
//   Free Agent  → release_to_fa(thisTeam, player)            (vacate slot → off-roster pool)
//   Sign        → sign_from_fa(player, otherTeam)            (install FA into the other team's block)
//   Delete      → delete_player(player)                      (whole-cluster removal)
//   Trade Pick  → trade_draft_pick(record, otherTeam)        (qHav owner rewrite; provenance kept)
//   Trade       → trade_slots(aTeam, aPlayer, bTeam, bPlayer)(in-place swap of both columns' picks)
//
// "Free agent" and "sign" are the Spike-S1 occupancy model (off-roster = an indexed player in no
// eGlu slot; docs/audits/roster-pools.md). Unlike the reference app there is no staging step —
// every action commits an audited, undoable transaction immediately (undo/redo in the toolbar;
// nothing touches disk until Save).
// ---------------------------------------------------------------------------

type Side = "A" | "B";
type ColumnMode = "players" | "freeAgents" | "picks";
type TxAction = "move" | "copy" | "freeAgent" | "sign" | "delete" | "tradePick";

interface PendingConfirm {
  action: "freeAgent" | "delete";
  player: PlayerSummaryDto;
  fromTeam: number | null;
}

export function TransactionsPage() {
  const qc = useQueryClient();
  const { project, update } = useProject();
  const projectId = project!.project_id;

  const teams = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });
  const players = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  const freeAgents = useQuery({
    queryKey: ["free-agents", projectId],
    queryFn: () => call("list_free_agents", { projectId }),
  });

  const [teamA, setTeamA] = useState<number | null>(null);
  const [teamB, setTeamB] = useState<number | null>(null);
  // Each column reports its currently-selected player up so the parent can offer Trade (a swap
  // needs both columns' picks). `null` when the column has no player selected (or is on picks).
  const [selA, setSelA] = useState<PlayerSummaryDto | null>(null);
  const [selB, setSelB] = useState<PlayerSummaryDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState<PendingConfirm | null>(null);

  const allTeams = teams.data ?? [];
  const allPlayers = players.data ?? [];
  const pool = freeAgents.data ?? [];

  const leagues = useMemo(() => {
    const seen = new Map<number, string>();
    for (const t of allTeams) {
      const code = t.league ?? 0;
      if (!seen.has(code)) seen.set(code, t.league_label ?? `League ${code}`);
    }
    return [...seen.entries()]
      .sort((a, b) => a[0] - b[0])
      .map(([id, label]) => ({ id, label }));
  }, [allTeams]);

  const refresh = () => { void refreshFields(qc, projectId, update).catch(e => setError(String(e))); };

  // Show moves in both rosters immediately, retaining snapshots for validation failures.
  async function projectRoster(moves: Array<{ player: PlayerSummaryDto; team: number | null }>) {
    await Promise.all([qc.cancelQueries({ queryKey: ["players", projectId] }), qc.cancelQueries({ queryKey: ["free-agents", projectId] })]);
    const before = qc.getQueryData<PlayerSummaryDto[]>(["players", projectId]);
    const beforePool = qc.getQueryData<PlayerSummaryDto[]>(["free-agents", projectId]);
    qc.setQueryData<PlayerSummaryDto[]>(["players", projectId], rows => rows?.map(p => {
      const move = moves.find(m => m.player.player_id === p.player_id);
      return move ? { ...p, team_id: move.team } : p;
    }));
    qc.setQueryData<PlayerSummaryDto[]>(["free-agents", projectId], rows => [
      ...(rows ?? []).filter(p => !moves.some(m => m.player.player_id === p.player_id)),
      ...moves.filter(m => m.team === null).map(m => ({ ...m.player, team_id: null })),
    ]);
    return () => { qc.setQueryData(["players", projectId], before); qc.setQueryData(["free-agents", projectId], beforePool); };
  }

  async function runAction(
    action: TxAction,
    subject: { player?: PlayerSummaryDto; pick?: DraftPickDto },
    fromTeam: number | null,
    otherTeam: number | null,
  ) {
    setBusy(true);
    setError(null);
    let rollback: (() => void) | undefined;
    try {
      if (subject.player && (action === "freeAgent" || (["move", "sign"].includes(action) && otherTeam !== null))) {
        rollback = await projectRoster([{ player: subject.player, team: action === "freeAgent" ? null : otherTeam }]);
      }
      const pid = subject.player?.player_id;
      switch (action) {
        case "move":
          if (pid == null || fromTeam === null || otherTeam === null) return;
          await queueFieldWrite(projectId, () => call("move_player", { projectId, player: pid, fromTeam, toTeam: otherTeam }));
          break;
        case "copy":
          if (pid == null || otherTeam === null) return;
          await call("clone_player", { projectId, sourcePlayer: pid, toTeam: otherTeam });
          break;
        case "freeAgent":
          if (pid == null || fromTeam === null) return;
          await queueFieldWrite(projectId, () => call("release_to_fa", { projectId, fromTeam, player: pid }));
          break;
        case "sign":
          if (pid == null || otherTeam === null) return;
          await queueFieldWrite(projectId, () => call("sign_from_fa", { projectId, player: pid, toTeam: otherTeam }));
          break;
        case "delete":
          if (pid == null) return;
          await call("delete_player", { projectId, player: pid });
          break;
        case "tradePick":
          if (!subject.pick || otherTeam === null) return;
          await call("trade_draft_pick", {
            projectId,
            record: subject.pick.record,
            toTeam: otherTeam,
          });
          break;
      }
      await refresh();
    } catch (e) {
      rollback?.();
      setError(String(e));
    } finally {
      setBusy(false);
      setPending(null);
    }
  }

  function handleAction(
    side: Side,
    action: TxAction,
    subject: { player?: PlayerSummaryDto; pick?: DraftPickDto },
  ) {
    const fromTeam = side === "A" ? teamA : teamB;
    const otherTeam = side === "A" ? teamB : teamA;
    // Confirm the destructive / harder-to-notice actions; run the rest directly.
    if ((action === "delete" || action === "freeAgent") && subject.player) {
      setPending({ action, player: subject.player, fromTeam });
    } else {
      void runAction(action, subject, fromTeam, otherTeam);
    }
  }

  // Trade = swap the two columns' selected occupants in place (both must be rostered, on different
  // teams). Acts on both selections at once, so it lives at the parent, not a column.
  async function runTrade() {
    if (selA?.team_id == null || selB?.team_id == null || selA.team_id === selB.team_id) return;
    setBusy(true);
    setError(null);
    const a = selA, b = selB;
    let rollback: (() => void) | undefined;
    try {
      rollback = await projectRoster([{ player: a, team: b.team_id }, { player: b, team: a.team_id }]);
      await queueFieldWrite(projectId, () => call("trade_slots", {
        projectId,
        aTeam: a.team_id!,
        aPlayer: a.player_id,
        bTeam: b.team_id!,
        bPlayer: b.player_id,
      }));
      await refresh();
    } catch (e) {
      rollback?.();
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const canTrade =
    selA?.team_id != null && selB?.team_id != null && selA.team_id !== selB.team_id && !busy;

  if (teams.isLoading || players.isLoading) return <Spinner label="Loading rosters…" />;
  if (teams.isError) return <p className="text-rose-400">{String(teams.error)}</p>;

  const playerName = (p: PlayerSummaryDto) =>
    `${p.first_name} ${p.last_name}`.trim() || `Player #${p.player_id}`;

  return (
    <div className="flex h-full flex-col gap-3">
      <div className="flex flex-wrap items-baseline justify-between gap-3">
        <h1 className="text-base font-semibold text-slate-100">Transactions</h1>
        <p className="text-xs text-slate-500">
          Move, copy, release, or delete players — and trade draft picks — between two teams.
          Every action is an audited, undoable edit; nothing touches disk until you save.
        </p>
      </div>
      {error && <p className="text-xs text-rose-400">{error}</p>}

      <div className="flex min-h-0 flex-1 gap-4">
        <TransactionColumn
          label="Team A"
          projectId={projectId}
          leagues={leagues}
          teams={allTeams}
          allPlayers={allPlayers}
          pool={pool}
          poolLoading={freeAgents.isLoading}
          selectedTeam={teamA}
          onSelectTeam={setTeamA}
          otherTeam={teamB}
          busy={busy}
          onSelectPlayer={setSelA}
          onAction={(action, subject) => handleAction("A", action, subject)}
        />
        <div className="flex flex-col items-center justify-center">
          <ActionButton
            label="⇄ Trade"
            tone="primary"
            disabled={!canTrade}
            title={
              selA?.team_id == null || selB?.team_id == null
                ? "Select a rostered player in each column"
                : selA.team_id === selB.team_id
                  ? "Pick players on two different teams"
                  : "Swap these two players' roster slots"
            }
            onClick={() => void runTrade()}
          />
        </div>
        <TransactionColumn
          label="Team B"
          projectId={projectId}
          leagues={leagues}
          teams={allTeams}
          allPlayers={allPlayers}
          pool={pool}
          poolLoading={freeAgents.isLoading}
          selectedTeam={teamB}
          onSelectTeam={setTeamB}
          otherTeam={teamA}
          busy={busy}
          onSelectPlayer={setSelB}
          onAction={(action, subject) => handleAction("B", action, subject)}
        />
      </div>

      {pending && (
        <ConfirmModal
          title={
            pending.action === "delete"
              ? "Delete player from the database?"
              : "Send player to Free Agents?"
          }
          body={
            pending.action === "delete" ? (
              <span>
                Remove <strong>{playerName(pending.player)}</strong> and their whole cluster
                (bio, ratings, roster slots) from the database. Undoable until you save.
              </span>
            ) : (
              <span>
                Release <strong>{playerName(pending.player)}</strong> from their team to Free
                Agents. Undoable until you save.
              </span>
            )
          }
          confirmLabel={pending.action === "delete" ? "Delete" : "Release"}
          danger={pending.action === "delete"}
          onConfirm={() =>
            void runAction(pending.action, { player: pending.player }, pending.fromTeam, null)
          }
          onCancel={() => setPending(null)}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// One side of the panel (rendered twice, mirrored)
// ---------------------------------------------------------------------------

function teamLabel(t: TeamSummaryDto): string {
  const name = t.name.trim() || `Team ${t.team_id}`;
  return t.abbrev ? `${name} (${t.abbrev})` : name;
}

function TransactionColumn({
  label,
  projectId,
  leagues,
  teams,
  allPlayers,
  pool,
  poolLoading,
  selectedTeam,
  onSelectTeam,
  otherTeam,
  busy,
  onSelectPlayer,
  onAction,
}: {
  label: string;
  projectId: number;
  leagues: Array<{ id: number; label: string }>;
  teams: TeamSummaryDto[];
  allPlayers: PlayerSummaryDto[];
  pool: PlayerSummaryDto[];
  poolLoading: boolean;
  selectedTeam: number | null;
  onSelectTeam: (teamId: number | null) => void;
  otherTeam: number | null;
  busy: boolean;
  onSelectPlayer: (player: PlayerSummaryDto | null) => void;
  onAction: (action: TxAction, subject: { player?: PlayerSummaryDto; pick?: DraftPickDto }) => void;
}) {
  const [league, setLeague] = useState<number | null>(leagues[0]?.id ?? null);
  const [mode, setMode] = useState<ColumnMode>("players");
  const [selectedPlayerId, setSelectedPlayerId] = useState<number | null>(null);
  const [selectedPickRecord, setSelectedPickRecord] = useState<number | null>(null);

  const picksQ = useQuery({
    queryKey: ["draft-picks", projectId, selectedTeam],
    queryFn: () => call("team_draft_picks", { projectId, teamId: selectedTeam as number }),
    enabled: selectedTeam !== null && mode === "picks",
  });

  const teamsInLeague = useMemo(
    () =>
      teams
        .filter((t) => league === null || (t.league ?? 0) === league)
        .sort((a, b) => teamLabel(a).localeCompare(teamLabel(b))),
    [teams, league],
  );

  const roster = useMemo(
    () =>
      selectedTeam === null
        ? []
        : allPlayers
            .filter((p) => p.team_id === selectedTeam)
            .sort(
              (a, b) =>
                (a.jersey_number ?? 999) - (b.jersey_number ?? 999) ||
                a.last_name.localeCompare(b.last_name),
            ),
    [allPlayers, selectedTeam],
  );
  // The free-agent pool, sorted by name (no team/jersey to sort on).
  const freeAgents = useMemo(
    () => [...pool].sort((a, b) => a.last_name.localeCompare(b.last_name)),
    [pool],
  );
  // The player list the column currently shows (roster or the pool).
  const shownPlayers = mode === "freeAgents" ? freeAgents : roster;
  const picks = picksQ.data ?? [];

  // Drop selections whenever the visible list changes.
  useEffect(() => {
    setSelectedPlayerId(null);
    setSelectedPickRecord(null);
  }, [selectedTeam, mode]);

  const selectedPlayer = shownPlayers.find((p) => p.player_id === selectedPlayerId) ?? null;
  const selectedPick = picks.find((p) => p.record === selectedPickRecord) ?? null;
  const hasTarget = otherTeam !== null && otherTeam !== selectedTeam;

  // Keep the parent's Trade-eligibility view in sync with this column's selection.
  useEffect(() => {
    onSelectPlayer(selectedPlayer);
  }, [selectedPlayer, onSelectPlayer]);

  return (
    <div className="flex min-w-0 flex-1 flex-col rounded-lg border border-slate-800 bg-slate-900/40">
      <div className="border-b border-slate-800 px-3 py-2">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-sm font-semibold text-slate-100">{label}</h2>
          <div className="flex rounded border border-slate-700 text-[10px]">
            {(["players", "freeAgents", "picks"] as const).map((m) => (
              <button
                key={m}
                type="button"
                onClick={() => setMode(m)}
                className={
                  "px-2 py-0.5 uppercase tracking-wide " +
                  (mode === m
                    ? "bg-slate-700/70 text-slate-100"
                    : "text-slate-400 hover:text-slate-200")
                }
              >
                {m === "players" ? "Players" : m === "freeAgents" ? "Free Agents" : "Draft Picks"}
              </button>
            ))}
          </div>
        </div>
        <div className="mt-2 flex gap-2">
          <select
            aria-label={`${label} league`}
            className="min-w-0 flex-1 rounded border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-xs text-slate-200"
            value={league ?? ""}
            onChange={(e) => {
              setLeague(e.target.value === "" ? null : Number(e.target.value));
              onSelectTeam(null);
            }}
          >
            {leagues.map((l) => (
              <option key={l.id} value={l.id}>
                {l.label}
              </option>
            ))}
          </select>
          <select
            aria-label={`${label} team`}
            className="min-w-0 flex-[2] rounded border border-slate-700 bg-slate-950 px-2 py-1 font-mono text-xs text-slate-200"
            value={selectedTeam ?? ""}
            onChange={(e) => onSelectTeam(e.target.value === "" ? null : Number(e.target.value))}
          >
            <option value="">— select team —</option>
            {teamsInLeague.map((t) => (
              <option key={t.team_id} value={t.team_id}>
                {teamLabel(t)}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="min-h-[14rem] flex-1 overflow-auto">
        {mode === "freeAgents" ? (
          poolLoading ? (
            <Spinner label="Loading free agents…" />
          ) : freeAgents.length === 0 ? (
            <Hint>No free agents available.</Hint>
          ) : (
            <RosterTable
              roster={freeAgents}
              selectedId={selectedPlayerId}
              onSelect={setSelectedPlayerId}
            />
          )
        ) : selectedTeam === null ? (
          <Hint>Pick a league and team to view its {mode === "players" ? "roster" : "picks"}.</Hint>
        ) : mode === "players" ? (
          roster.length === 0 ? (
            <Hint>No players on this team.</Hint>
          ) : (
            <RosterTable
              roster={roster}
              selectedId={selectedPlayerId}
              onSelect={setSelectedPlayerId}
            />
          )
        ) : picksQ.isLoading ? (
          <Spinner label="Loading picks…" />
        ) : picks.length === 0 ? (
          <Hint>This team owns no future draft picks.</Hint>
        ) : (
          <PicksTable
            picks={picks}
            selectedRecord={selectedPickRecord}
            onSelect={setSelectedPickRecord}
          />
        )}
      </div>

      <div className="flex flex-wrap gap-2 border-t border-slate-800 px-3 py-2">
        {mode === "freeAgents" ? (
          <ActionButton
            label="Sign"
            tone="primary"
            disabled={busy || !selectedPlayer || otherTeam === null}
            title={
              !selectedPlayer
                ? "Select a free agent"
                : otherTeam === null
                  ? "Select a team in the other column to sign them to"
                  : "Sign this free agent to the other column's team"
            }
            onClick={() => selectedPlayer && onAction("sign", { player: selectedPlayer })}
          />
        ) : mode === "players" ? (
          <>
            <ActionButton
              label="Move"
              tone="primary"
              disabled={busy || !selectedPlayer || !hasTarget}
              title={
                !selectedPlayer
                  ? "Select a player"
                  : !hasTarget
                    ? "Select a team in the other column"
                    : "Move this player to the other column's team"
              }
              onClick={() => selectedPlayer && onAction("move", { player: selectedPlayer })}
            />
            <ActionButton
              label="Copy"
              tone="default"
              disabled={busy || !selectedPlayer || !hasTarget}
              title={
                !selectedPlayer
                  ? "Select a player"
                  : !hasTarget
                    ? "Select a team in the other column"
                    : "Clone this player onto the other team (keeps them here)"
              }
              onClick={() => selectedPlayer && onAction("copy", { player: selectedPlayer })}
            />
            <ActionButton
              label="Free Agent"
              tone="default"
              disabled={busy || !selectedPlayer}
              title={selectedPlayer ? "Release this player to Free Agents" : "Select a player"}
              onClick={() => selectedPlayer && onAction("freeAgent", { player: selectedPlayer })}
            />
            <ActionButton
              label="Delete"
              tone="danger"
              disabled={busy || !selectedPlayer}
              title={
                selectedPlayer
                  ? "Remove this player's whole cluster from the database"
                  : "Select a player"
              }
              onClick={() => selectedPlayer && onAction("delete", { player: selectedPlayer })}
            />
          </>
        ) : (
          <ActionButton
            label="Trade Pick"
            tone="primary"
            disabled={busy || !selectedPick || !hasTarget}
            title={
              !selectedPick
                ? "Select a pick"
                : !hasTarget
                  ? "Select a team in the other column"
                  : "Send this pick to the other column's team (origin stays on record)"
            }
            onClick={() => selectedPick && onAction("tradePick", { pick: selectedPick })}
          />
        )}
      </div>
    </div>
  );
}

function Hint({ children }: { children: ReactNode }) {
  return <p className="px-3 py-6 text-center text-xs text-slate-500">{children}</p>;
}

function RosterTable({
  roster,
  selectedId,
  onSelect,
}: {
  roster: PlayerSummaryDto[];
  selectedId: number | null;
  onSelect: (id: number) => void;
}) {
  return (
    <table className="w-full text-left text-xs">
      <thead className="sticky top-0 border-b border-slate-800 bg-slate-900/90 text-[10px] uppercase tracking-wider text-slate-500">
        <tr>
          <th className="px-2 py-1.5 font-medium">#</th>
          <th className="px-2 py-1.5 font-medium">First</th>
          <th className="px-2 py-1.5 font-medium">Last</th>
          <th className="px-2 py-1.5 font-medium">Pos</th>
          <th className="px-2 py-1.5 text-right font-medium">OVR</th>
          <th className="px-2 py-1.5 font-medium">Style</th>
          <th className="px-2 py-1.5 font-medium">Status</th>
        </tr>
      </thead>
      <tbody className="divide-y divide-slate-900">
        {roster.map((p) => {
          const isSel = p.player_id === selectedId;
          return (
            <tr
              key={p.player_id}
              onClick={() => onSelect(p.player_id)}
              className={
                "cursor-pointer " +
                (isSel ? "bg-sky-900/40 text-slate-100" : "text-slate-300 hover:bg-slate-800/40")
              }
            >
              <td className="px-2 py-1 font-mono tabular-nums text-slate-400">
                {p.jersey_number ?? "—"}
              </td>
              <td className="px-2 py-1">{p.first_name || "—"}</td>
              <td className="px-2 py-1 font-medium">
                {p.last_name || "—"}
              </td>
              <td className="px-2 py-1 text-slate-400">{p.position_label || "—"}</td>
              <td
                className={
                  "px-2 py-1 text-right font-mono tabular-nums " + ratingToneClass(p.overall)
                }
              >
                {p.overall ?? "—"}
              </td>
              <td className="px-2 py-1 text-slate-400">{p.style_label ?? "—"}</td>
              <td className="px-2 py-1">
                {p.is_active === false ? (
                  <Pill tone="warn">Inactive</Pill>
                ) : (
                  <Pill tone="neutral">Active</Pill>
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

const ORDINAL = ["", "1st", "2nd", "3rd", "4th", "5th", "6th", "7th"];

function PicksTable({
  picks,
  selectedRecord,
  onSelect,
}: {
  picks: DraftPickDto[];
  selectedRecord: number | null;
  onSelect: (record: number) => void;
}) {
  return (
    <table className="w-full table-fixed text-left text-xs">
      <thead className="sticky top-0 border-b border-slate-800 bg-slate-900/90 text-[10px] uppercase tracking-wider text-slate-500">
        <tr>
          <th className="w-16 px-2 py-1.5 font-medium">Year</th>
          <th className="w-20 px-2 py-1.5 font-medium">Round</th>
          <th className="w-20 px-2 py-1.5 text-right font-medium">Overall</th>
          <th className="px-2 py-1.5 font-medium">Origin</th>
        </tr>
      </thead>
      <tbody className="divide-y divide-slate-900">
        {picks.map((p) => {
          const isSel = p.record === selectedRecord;
          return (
            <tr
              key={p.record}
              onClick={() => onSelect(p.record)}
              className={
                "cursor-pointer " +
                (isSel ? "bg-sky-900/40 text-slate-100" : "text-slate-300 hover:bg-slate-800/40")
              }
            >
              <td className="px-2 py-1 font-mono tabular-nums">{p.year}</td>
              <td className="px-2 py-1">{ORDINAL[p.round] ?? `R${p.round}`}</td>
              <td className="px-2 py-1 text-right font-mono tabular-nums text-slate-400">
                #{p.overall}
              </td>
              <td className="truncate px-2 py-1 text-slate-400">
                {p.via_trade ? `via trade — ${p.original_owner_name ?? "unknown"}` : "Own pick"}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function ActionButton({
  label,
  tone,
  disabled,
  title,
  onClick,
}: {
  label: string;
  tone: "primary" | "danger" | "default";
  disabled: boolean;
  title: string;
  onClick: () => void;
}) {
  const toneClass =
    tone === "primary"
      ? "border-sky-700 bg-sky-900/50 text-sky-100 hover:bg-sky-800/60"
      : tone === "danger"
        ? "border-rose-800 bg-rose-950/50 text-rose-200 hover:bg-rose-900/60"
        : "border-slate-700 bg-slate-800/60 text-slate-200 hover:bg-slate-700/60";
  return (
    <button
      type="button"
      disabled={disabled}
      title={title}
      onClick={onClick}
      className={`rounded border px-3 py-1 text-xs font-medium disabled:cursor-not-allowed disabled:opacity-40 ${toneClass}`}
    >
      {label}
    </button>
  );
}

/** Minimal confirm dialog for the destructive transaction verbs. */
function ConfirmModal({
  title,
  body,
  confirmLabel,
  danger,
  onConfirm,
  onCancel,
}: {
  title: string;
  body: ReactNode;
  confirmLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onCancel]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/70 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      onClick={onCancel}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-[26rem] rounded-lg border border-slate-700 bg-slate-900 p-4 shadow-2xl"
      >
        <h2 className="text-sm font-semibold text-slate-100">{title}</h2>
        <p className="mt-2 text-xs leading-relaxed text-slate-300">{body}</p>
        <div className="mt-4 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="rounded border border-slate-700 px-3 py-1 text-xs text-slate-300 hover:bg-slate-800"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className={
              "rounded border px-3 py-1 text-xs font-medium " +
              (danger
                ? "border-rose-800 bg-rose-950/60 text-rose-200 hover:bg-rose-900/60"
                : "border-sky-700 bg-sky-900/50 text-sky-100 hover:bg-sky-800/60")
            }
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
