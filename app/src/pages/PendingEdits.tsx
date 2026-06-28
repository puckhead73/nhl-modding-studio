import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { Button, Card } from "../components/primitives";

export function PendingEdits() {
  const { project, dbPath, update } = useProject();
  const qc = useQueryClient();
  const p = project!;
  const projectId = p.project_id;
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
    <div className="max-w-3xl space-y-3">
      <h2 className="text-lg font-semibold text-slate-100">Pending edits</h2>

      <Card title="Session">
        <p className="text-sm text-slate-200">
          Status:{" "}
          {p.dirty ? (
            <b className="text-amber-300">unsaved changes</b>
          ) : (
            <span className="text-slate-400">saved</span>
          )}{" "}
          · undo depth {p.undo_depth} · redo depth {p.redo_depth}
        </p>
        <div className="mt-3 flex items-center gap-2">
          <Button variant="secondary" onClick={() => undo.mutate()} disabled={busy || p.undo_depth === 0}>
            Undo
          </Button>
          <Button variant="secondary" onClick={() => redo.mutate()} disabled={busy || p.redo_depth === 0}>
            Redo
          </Button>
          <Button onClick={() => dbPath && save.mutate(dbPath)} disabled={busy || !p.dirty || !dbPath}>
            Save (in place)
          </Button>
        </div>
      </Card>

      <Card title="Save as / Pack" subtitle={`Container projects repack automatically (${p.source}).`}>
        <div className="flex items-center gap-2">
          <input
            value={saveAsPath}
            onChange={(e) => setSaveAsPath(e.target.value)}
            placeholder="output path"
            className="flex-1 rounded-md border border-slate-700 bg-slate-950 px-2 py-1.5 text-sm text-slate-100 placeholder-slate-600 focus:border-sky-600 focus:outline-none"
          />
          <Button variant="secondary" onClick={() => save.mutate(saveAsPath)} disabled={busy || !saveAsPath}>
            Save as
          </Button>
        </div>
      </Card>

      {save.isSuccess && (
        <p className="text-sm text-emerald-400">Wrote {Number(save.data).toLocaleString()} bytes.</p>
      )}
      {(undo.isError || redo.isError || save.isError) && (
        <p className="text-sm text-rose-400">{String(undo.error || redo.error || save.error)}</p>
      )}
    </div>
  );
}
