import { useState } from "react";
import { createPortal } from "react-dom";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { call } from "../lib/client";
import { useProject } from "../lib/project";
import { useSettings } from "../lib/settings";
import { notifyProjectSaved } from "../lib/saveNotice";
import { Button } from "./primitives";
import type { ProjectOpenDto } from "../bindings/ProjectOpenDto";

export function ProjectFolderField() {
  const { project, info, open, update, updateInfo } = useProject();
  const { rememberProject } = useSettings();
  const qc = useQueryClient();
  const [draft, setDraft] = useState<string | null>(null);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const adopt = (r: ProjectOpenDto) => {
    open(r.summary, r.project.working_main_db ?? r.project.working_dir, r.discovery, null, r.project);
    rememberProject(r.project.dir);
  };
  const change = useMutation({ mutationFn: async () => {
    if (!project || !info) return;
    const projectId = project.project_id;
    const oldDir = info.dir;
    const saved = await call("project_save", { projectId });
    if (!saved.saved && !saved.nothing_to_save) throw new Error("The project could not be saved. Its folder has not changed.");
    update(saved.summary);
    updateInfo(await call("project_info", { projectId }), undefined, projectId);
    notifyProjectSaved(projectId);
    const destination = await call("project_copy_to", { projectId, destination: draft!.trim() });
    // Do not abandon an active operation or lose the original on a failed copy/open.
    const closed = await call("close_project", { projectId });
    if (closed.outcome !== "closed" && closed.outcome !== "already_closed") {
      throw new Error(`The copy is saved at ${destination}, but the current session is still busy. ${closed.reason ?? ""}`);
    }
    try {
      adopt(await call("project_open", { projectDir: destination }));
    } catch (error) {
      adopt(await call("project_open", { projectDir: oldDir }));
      throw error;
    }
    setDraft(null);
    await qc.invalidateQueries();
  } });
  if (!info) return null;
  const browse = async () => {
    try {
      setPickerError(null);
      const parent = await openDialog({ directory: true, multiple: false, title: "Choose where to store this project" });
      if (typeof parent === "string") {
        const name = info.dir.replace(/[\\/]+$/, "").split(/[\\/]/).pop()!;
        setDraft(`${parent.replace(/[\\/]+$/, "")}/${name}`);
      }
    } catch (error) { setPickerError(String(error)); }
  };
  return <div className="mt-4 space-y-2">
    <label className="block text-xs text-slate-300" htmlFor="overview-project-folder">Project folder</label>
    <div className="flex gap-2">
      <input id="overview-project-folder" value={draft ?? info.dir} onChange={e => setDraft(e.target.value)} disabled={change.isPending}
        className="min-w-0 flex-1 rounded-md border border-slate-700 bg-slate-950 px-3 py-2 text-xs text-slate-100" />
      <Button type="button" variant="secondary" disabled={change.isPending} onClick={() => void browse()}>Browse…</Button>
      <Button type="button" disabled={change.isPending || !draft?.trim() || draft.trim() === info.dir} onClick={() => change.mutate()}>Change folder</Button>
    </div>
    {draft != null && draft !== info.dir && <p className="text-xs text-slate-400">Saves and opens the project in a new folder. Your previous copy is kept.</p>}
    {(change.isError || pickerError) && <p role="alert" className="text-xs text-rose-300">{String(change.error ?? pickerError)}</p>}
    {change.isPending && createPortal(<div role="status" className="fixed inset-0 z-[110] grid place-items-center bg-slate-950/90 text-slate-100">Saving and changing project folder…</div>, document.body)}
  </div>;
}
