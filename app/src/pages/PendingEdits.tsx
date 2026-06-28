import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import styles from "./PendingEdits.module.css";

export function PendingEdits() {
  const { project, dbPath, update } = useProject();
  const qc = useQueryClient();
  const projectId = project!.project_id;
  const [saveAsPath, setSaveAsPath] = useState(dbPath ?? "");

  const refresh = async () => {
    await qc.invalidateQueries();
    update(await call("project_summary", { projectId }));
  };

  const undo = useMutation({ mutationFn: () => call("undo", { projectId }), onSuccess: refresh });
  const redo = useMutation({ mutationFn: () => call("redo", { projectId }), onSuccess: refresh });
  const save = useMutation({
    mutationFn: (outPath: string) => call("save_project", { projectId, outPath }),
    onSuccess: refresh,
  });

  const busy = undo.isPending || redo.isPending || save.isPending;

  return (
    <div className={styles.wrap}>
      <h2>Pending edits</h2>

      <section className={styles.card}>
        <p>
          Status: {project!.dirty ? <b className={styles.dirty}>unsaved changes</b> : "saved"} ·
          undo depth {project!.undo_depth} · redo depth {project!.redo_depth}
        </p>
        <div className={styles.actions}>
          <button onClick={() => undo.mutate()} disabled={busy || project!.undo_depth === 0}>
            Undo
          </button>
          <button onClick={() => redo.mutate()} disabled={busy || project!.redo_depth === 0}>
            Redo
          </button>
          <button
            className={styles.primary}
            onClick={() => dbPath && save.mutate(dbPath)}
            disabled={busy || !project!.dirty || !dbPath}
          >
            Save (in place)
          </button>
        </div>
      </section>

      <section className={styles.card}>
        <h3>Save as / Pack</h3>
        <p className={styles.muted}>
          Writes the current database to a new path. Container projects are repacked automatically
          ({project!.source}).
        </p>
        <div className={styles.actions}>
          <input
            className={styles.path}
            value={saveAsPath}
            onChange={(e) => setSaveAsPath(e.target.value)}
            placeholder="output path"
          />
          <button onClick={() => save.mutate(saveAsPath)} disabled={busy || !saveAsPath}>
            Save as
          </button>
        </div>
      </section>

      {save.isSuccess && <p className={styles.ok}>Wrote {Number(save.data).toLocaleString()} bytes.</p>}
      {(undo.isError || redo.isError || save.isError) && (
        <p className={styles.err}>{String(undo.error || redo.error || save.error)}</p>
      )}
    </div>
  );
}
