import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import styles from "./detail.module.css";

export function TeamDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const teamId = Number(id);

  const view = useQuery({
    queryKey: ["team", projectId, teamId],
    queryFn: () => call("team_view", { projectId, teamId }),
    enabled: Number.isFinite(teamId),
  });

  if (view.isLoading) return <p className={styles.muted}>Loading…</p>;
  if (view.isError) return <p className={styles.err}>{String(view.error)}</p>;
  const team = view.data;
  if (!team) return <p className={styles.muted}>Select a team.</p>;

  return (
    <div className={styles.wrap}>
      <h2 className={styles.title}>
        Team #{team.team_id} <span className={styles.tag}>{team.roster_count} on roster</span>
      </h2>

      <section className={styles.card}>
        <h3>Identity</h3>
        {team.identity.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={team.record} field={f} />
        ))}
      </section>

      <section className={styles.card}>
        <h3>Arena</h3>
        {team.arena ? (
          <p>
            <Link className={styles.link} to={`/app/arenas/${team.arena.arena_id}`}>
              {team.arena.name || `Arena ${team.arena.arena_id}`}
            </Link>
            {team.arena.city && ` — ${team.arena.city}`}
          </p>
        ) : (
          <p className={styles.muted}>No arena resolved.</p>
        )}
      </section>

      <details className={styles.card}>
        <summary>All team fields ({team.all_fields.length})</summary>
        {team.all_fields.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={team.record} field={f} />
        ))}
      </details>
    </div>
  );
}
