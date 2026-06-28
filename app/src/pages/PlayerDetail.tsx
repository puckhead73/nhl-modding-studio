import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import styles from "./PlayerDetail.module.css";

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

  if (view.isLoading) return <p className={styles.muted}>Loading…</p>;
  if (view.isError) return <p className={styles.err}>{String(view.error)}</p>;
  const player = view.data;
  if (!player) return <p className={styles.muted}>Select a player.</p>;

  return (
    <div className={styles.wrap}>
      <h2 className={styles.title}>
        Player #{player.player_id} <span className={styles.kind}>{player.kind}</span>
      </h2>

      <section className={styles.card}>
        <h3>Identity</h3>
        {player.identity.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={player.bio_record} field={f} />
        ))}
      </section>

      <section className={styles.card}>
        <h3>Roster memberships</h3>
        {player.memberships.length === 0 && <p className={styles.muted}>Not on any roster.</p>}
        {player.memberships.map((m, i) => (
          <div key={i} className={styles.membership}>
            <span>Team {m.team_id}</span>
            <span>Jersey {m.jersey ?? "—"}</span>
            <span>Captaincy {m.captaincy ?? "—"}</span>
            <span>Status {m.roster_status ?? "—"}</span>
          </div>
        ))}
      </section>

      <details className={styles.card}>
        <summary>Ratings ({player.ratings.length})</summary>
        {player.ratings_record === null && player.ratings.length > 0 && (
          <p className={styles.muted}>No companion record — ratings shown read-only.</p>
        )}
        {player.ratings.map((f) => (
          <FieldRow
            key={f.studio_field}
            projectId={projectId}
            record={player.ratings_record ?? player.bio_record}
            field={{ ...f, editable: f.editable && player.ratings_record !== null }}
          />
        ))}
      </details>

      <details className={styles.card}>
        <summary>All bio fields ({player.all_fields.length})</summary>
        {player.all_fields.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={player.bio_record} field={f} />
        ))}
      </details>
    </div>
  );
}
