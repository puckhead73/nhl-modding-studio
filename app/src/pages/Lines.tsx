import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Pill, Spinner } from "../components/primitives";

export function LinesPage() {
  const { project } = useProject();
  const projectId = project!.project_id;
  const [teamId, setTeamId] = useState<number | null>(null);

  const teams = useQuery({
    queryKey: ["teams", projectId],
    queryFn: () => call("list_teams", { projectId }),
  });
  const lines = useQuery({
    queryKey: ["lines", projectId, teamId],
    queryFn: () => call("lines_view", { projectId, teamId: teamId! }),
    enabled: teamId !== null,
  });

  return (
    <div className="max-w-5xl space-y-3">
      <h2 className="text-lg font-semibold text-slate-100">Depth chart</h2>
      <div className="flex items-center gap-3">
        <select
          value={teamId ?? ""}
          onChange={(e) => setTeamId(e.target.value ? Number(e.target.value) : null)}
          className="rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 focus:border-sky-600 focus:outline-none"
        >
          <option value="">Select a team…</option>
          {(teams.data ?? []).map((t) => (
            <option key={t.team_id} value={t.team_id}>
              {t.name || `Team ${t.team_id}`} {t.abbrev && `(${t.abbrev})`}
            </option>
          ))}
        </select>
        <Pill tone="warn">situational slots are meta-derived (unverified)</Pill>
      </div>

      {lines.isLoading && <Spinner label="Loading…" />}
      {lines.isError && <p className="text-rose-400">{String(lines.error)}</p>}
      {lines.data && (
        <div className="overflow-auto rounded-lg border border-slate-800 bg-slate-900/40">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="bg-slate-800/60 text-xs text-slate-300">
                {["Player", "Pos", "#", "C", "Status", "Situational slots"].map((h) => (
                  <th key={h} className="border-b border-slate-800 px-3 py-1.5 text-left">
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {lines.data.entries.map((e) => (
                <tr key={e.slot_record} className="text-slate-200">
                  <td className="border-b border-slate-800/60 px-3 py-1.5">
                    {e.name || (e.player_id !== null ? `#${e.player_id}` : "—")}
                  </td>
                  <td className="border-b border-slate-800/60 px-3 py-1.5">{e.position_label}</td>
                  <td className="border-b border-slate-800/60 px-3 py-1.5">{e.jersey ?? "—"}</td>
                  <td className="border-b border-slate-800/60 px-3 py-1.5">{e.captaincy || ""}</td>
                  <td className="border-b border-slate-800/60 px-3 py-1.5">{e.roster_status ?? "—"}</td>
                  <td className="border-b border-slate-800/60 px-3 py-1.5 font-mono text-xs text-slate-400">
                    {e.slots.join(", ")}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {lines.data.entries.length === 0 && (
            <p className="px-3 py-3 text-sm text-slate-400">No roster entries for this team.</p>
          )}
        </div>
      )}
    </div>
  );
}
