import { useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { FieldRow } from "../components/FieldRow";
import styles from "./detail.module.css";

export function ArenaDetail() {
  const { id } = useParams<{ id: string }>();
  const { project } = useProject();
  const projectId = project!.project_id;
  const arenaId = Number(id);

  const view = useQuery({
    queryKey: ["arena", projectId, arenaId],
    queryFn: () => call("arena_view", { projectId, arenaId }),
    enabled: Number.isFinite(arenaId),
  });

  if (view.isLoading) return <p className={styles.muted}>Loading…</p>;
  if (view.isError) return <p className={styles.err}>{String(view.error)}</p>;
  const arena = view.data;
  if (!arena) return <p className={styles.muted}>Select an arena.</p>;

  return (
    <div className={styles.wrap}>
      <h2 className={styles.title}>Arena #{arena.arena_id}</h2>

      <section className={styles.card}>
        <h3>Identity</h3>
        {arena.identity.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={arena.record} field={f} />
        ))}
      </section>

      <details className={styles.card}>
        <summary>All arena fields ({arena.all_fields.length})</summary>
        {arena.all_fields.map((f) => (
          <FieldRow key={f.studio_field} projectId={projectId} record={arena.record} field={f} />
        ))}
      </details>
    </div>
  );
}
