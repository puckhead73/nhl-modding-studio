import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { call } from "../lib/client";
import { EmptyState, Spinner } from "./primitives";
import { ratingToneClass } from "../lib/ratingColor";
import type { PlayerSummaryDto } from "../bindings/PlayerSummaryDto";

const USD = new Intl.NumberFormat("en-US", {
  style: "currency",
  currency: "USD",
  maximumFractionDigits: 0,
});

const POS_ORDER = ["LW", "C", "RW", "D", "G"];

function playerName(p: PlayerSummaryDto): string {
  if (p.first_name && p.last_name) return `${p.last_name}, ${p.first_name}`;
  return p.last_name || p.first_name || `Player #${p.player_id}`;
}

/** A team's primary roster sorted position (LW→C→RW→D→G) then name — shared by the Contracts
 *  table (the reference layout). */
export function useRosterPlayers(projectId: number, teamId: number) {
  const q = useQuery({
    queryKey: ["players", projectId],
    queryFn: () => call("list_players", { projectId }),
  });
  const players = (q.data ?? [])
    .filter((p) => p.team_id === teamId)
    .sort((a, b) => {
      const ai = POS_ORDER.indexOf(a.position_label);
      const bi = POS_ORDER.indexOf(b.position_label);
      if (ai !== bi) return ai - bi;
      return playerName(a).localeCompare(playerName(b));
    });
  return { players, isLoading: q.isLoading, error: q.isError ? q.error : null };
}

/// The Contracts tab — the roster's contract table (cap hit / term / status per player) with the
/// team's total cap in the header. Driven entirely by `list_players` (the summary carries the
/// contract cells); per-player edits live on the Player Data tab.
export function ContractsTab({ projectId, teamId }: { projectId: number; teamId: number }) {
  const { players, isLoading, error } = useRosterPlayers(projectId, teamId);

  if (error) return <p className="text-rose-400">{String(error)}</p>;
  if (isLoading) return <Spinner label="Loading roster…" />;
  if (players.length === 0) {
    return <EmptyState title="No rostered players on this team" />;
  }

  const totalCap = players.reduce((sum, p) => sum + (p.cap_hit_dollars ?? 0), 0);

  return (
    <div className="space-y-3">
      <div className="flex items-baseline justify-between gap-3">
        <h2 className="text-sm font-semibold text-slate-100">Contracts</h2>
        <span className="text-xs text-slate-400">
          {players.length} {players.length === 1 ? "player" : "players"} · total cap{" "}
          {USD.format(totalCap)}
        </span>
      </div>
      <div className="overflow-x-auto rounded-lg border border-slate-800">
        <table className="w-full text-left text-xs">
          <thead className="border-b border-slate-800 bg-slate-900/60 text-[10px] uppercase tracking-wider text-slate-500">
            <tr>
              <th className="px-3 py-2 font-medium">Player</th>
              <th className="px-2 py-2 font-medium">Pos</th>
              <th className="px-2 py-2 text-right font-medium">OVR</th>
              <th className="px-3 py-2 text-right font-medium">Cap Hit</th>
              <th className="px-2 py-2 text-right font-medium">Term</th>
              <th className="px-3 py-2 font-medium">Status</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-900">
            {players.map((p) => (
              <tr key={p.player_id} className="hover:bg-slate-800/40">
                <td className="px-3 py-1.5 text-slate-100">
                  <Link
                    to={`/app/database/players/${p.player_id}`}
                    className="flex items-baseline gap-2 hover:text-sky-300"
                  >
                    <span className="inline-flex w-7 shrink-0 justify-end font-mono text-[11px] text-slate-500">
                      {p.jersey_number != null ? `#${p.jersey_number}` : "—"}
                    </span>
                    <span className="truncate">{playerName(p)}</span>
                  </Link>
                </td>
                <td className="px-2 py-1.5 text-slate-400">{p.position_label || "—"}</td>
                <td
                  className={
                    "px-2 py-1.5 text-right font-mono tabular-nums " + ratingToneClass(p.overall)
                  }
                >
                  {p.overall ?? "—"}
                </td>
                <td className="px-3 py-1.5 text-right font-mono tabular-nums text-slate-200">
                  {p.cap_hit_dollars ? USD.format(p.cap_hit_dollars) : "—"}
                </td>
                <td className="px-2 py-1.5 text-right tabular-nums text-slate-400">
                  {p.contract_years
                    ? `${p.contract_years} yr${p.contract_years === 1 ? "" : "s"}`
                    : "—"}
                </td>
                <td className="px-3 py-1.5 text-slate-400">
                  <span className="flex flex-wrap items-center gap-1.5">
                    {p.contract_status ?? "—"}
                    {p.is_two_way && (
                      <span className="rounded-full border border-slate-700 bg-slate-800/60 px-2 py-0.5 text-[10px] text-slate-300">
                        Two-Way
                      </span>
                    )}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
