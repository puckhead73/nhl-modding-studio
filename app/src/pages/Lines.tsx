import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import styles from "./Lines.module.css";

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
    <div className={styles.wrap}>
      <h2>Depth chart</h2>
      <div className={styles.controls}>
        <select
          value={teamId ?? ""}
          onChange={(e) => setTeamId(e.target.value ? Number(e.target.value) : null)}
        >
          <option value="">Select a team…</option>
          {(teams.data ?? []).map((t) => (
            <option key={t.team_id} value={t.team_id}>
              {t.name || `Team ${t.team_id}`} {t.abbrev && `(${t.abbrev})`}
            </option>
          ))}
        </select>
        <span className={styles.badge}>situational slots are meta-derived (unverified)</span>
      </div>

      {lines.isLoading && <p className={styles.muted}>Loading…</p>}
      {lines.isError && <p className={styles.err}>{String(lines.error)}</p>}
      {lines.data && (
        <table className={styles.table}>
          <thead>
            <tr>
              <th>Player</th>
              <th>Pos</th>
              <th>#</th>
              <th>C</th>
              <th>Status</th>
              <th>Situational slots</th>
            </tr>
          </thead>
          <tbody>
            {lines.data.entries.map((e) => (
              <tr key={e.slot_record}>
                <td>{e.name || (e.player_id !== null ? `#${e.player_id}` : "—")}</td>
                <td>{e.position_label}</td>
                <td>{e.jersey ?? "—"}</td>
                <td>{e.captaincy ? e.captaincy : ""}</td>
                <td>{e.roster_status ?? "—"}</td>
                <td className={styles.slots}>{e.slots.join(", ")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {lines.data && lines.data.entries.length === 0 && (
        <p className={styles.muted}>No roster entries for this team.</p>
      )}
    </div>
  );
}
